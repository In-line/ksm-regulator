/*
 * KSM Regulator
 * Copyright (c) 2019 Alik Aslanyan <cplusplus256@gmail.com>
 *
 *
 *    This file is part of KSM Regulator.
 *
 *    KSM Regulator is free software; you can redistribute it and/or modify it
 *    under the terms of the GNU General Public License as published by the
 *    Free Software Foundation; either version 3 of the License, or (at
 *    your option) any later version.
 *
 *    This program is distributed in the hope that it will be useful, but
 *    WITHOUT ANY WARRANTY; without even the implied warranty of
 *    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 *    General Public License for more details.
 *
 *    You should have received a copy of the GNU General Public License
 *    along with this program; if not, write to the Free Software Foundation,
 *    Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 */

use anyhow::Context;
use async_std::{
    fs::{File, OpenOptions},
    io::prelude::{ReadExt, WriteExt},
    path::PathBuf,
    task,
};
use futures::{
    future::{select, Either},
    FutureExt,
};
use interpolation::lerp;
use itertools::*;
use log::*;
use ordered_float::OrderedFloat;
use ordslice::Ext;
use serde_derive::Deserialize;
use std::cmp::{min, Ord};
use std::time::Duration;
use structopt::StructOpt;

type Result = anyhow::Result<()>;

async fn ctrl_c() {
    async_ctrlc::CtrlC::new()
        .expect("Can't initial CtrlC handler")
        .await
}

async fn set_ksm_run(opt: &Opt, value: bool) -> Result {
    trace!("Setting ksm run to {}", value);

    OpenOptions::new()
        .write(true)
        .open(&opt.ksm_run_file)
        .await
        .context("Can't open KSM run file.")?
        .write_all(if value { b"1" } else { b"0" })
        .await
        .context("Can't write to KSM run file")?;

    Ok(())
}

async fn set_ksm_sleep(opt: &Opt, value: f64) -> Result {
    set_ksm_run(opt, true).await?;

    let value = value as u64;
    trace!("Setting ksm sleep to {}", value);

    OpenOptions::new()
        .write(true)
        .open(&opt.ksm_sleep_millisecs_file)
        .await
        .context("Can't open KSM sleep time file.")?
        .write_all(value.to_string().as_bytes())
        .await
        .context("Can't write to KSM sleep time file")?;
    Ok(())
}

fn logarithmic_interpolation(a: &f64, b: &f64, scalar: &f64) -> f64 {
    a.powf(1.0 - *scalar) * b.powf(*scalar)
}

async fn process(opt: &Opt) -> Result {
    #[derive(Deserialize)]
    struct ConfigEntry {
        ksm_sleep_millisecs: f64,
        trigger_memory_above: f64,
    }

    debug!("Starting to process..");

    let (memory_map, sleep_map): (Vec<_>, Vec<_>) = serde_hjson::from_str::<Vec<ConfigEntry>>(&{
        let mut config = String::with_capacity(4096);
        File::open(&opt.config_file)
            .await
            .with_context(|| format!("Can't open file at path: {:?}", opt.config_file))?
            .read_to_string(&mut config)
            .await
            .with_context(|| format!("Can't read file at path: {:?}", opt.config_file))?;
        config
    })
    .context("Can't parse HJSON")?
    .into_iter()
    .map(|item| (item.trigger_memory_above, item.ksm_sleep_millisecs))
    .map(|(a, b)| (OrderedFloat::from(a), OrderedFloat::from(b)))
    .sorted_by(|(a, _), (b, _)| a.cmp(&b))
    .unzip();

    debug!("{:#?}", memory_map);
    debug!("{:#?}", sleep_map);

    loop {
        let mem_info = task::spawn_blocking(sys_info::mem_info)
            .await
            .context("Can't get memory info")?;

        let total_memory = mem_info.total as f64 / 1024.0;
        let used_memory = (mem_info.total - mem_info.avail) as f64 / 1024.0;

        let usage_percentage = used_memory / total_memory * 100.0;

        let index = min(
            memory_map.lower_bound(&usage_percentage.into()),
            memory_map.len() - 1,
        );

        trace!("Index: {}", index);
        info!(
            "Total memory: {:.2}M, Used memory: {:.2}M, Usage percentage: {:.2}",
            total_memory, used_memory, usage_percentage
        );

        if let Some(upper) = memory_map.get(index) {
            let final_sleep = if let Some(below) = memory_map.get(index - 1) {
                trace!("Below: {}, Upper: {}", below, upper);

                let ratio = {
                    let ratio: f64 = (usage_percentage - below.into_inner())
                        / (upper.into_inner() - below.into_inner());
                    if ratio >= 1.0 {
                        1.0 / ratio
                    } else {
                        ratio
                    }
                };

                trace!("Calculated ratio: {:.2}", ratio);

                if opt.linear_interpolation {
                    lerp::<f64>(&sleep_map[index - 1], &sleep_map[index], &ratio)
                } else {
                    logarithmic_interpolation(&sleep_map[index - 1], &sleep_map[index], &ratio)
                }
            } else {
                sleep_map[index].into_inner()
            };
            info!("Calculated sleep value: {}", final_sleep);

            set_ksm_sleep(opt, final_sleep).await?;
        } else {
            set_ksm_run(opt, false).await?;
        };
        task::sleep(Duration::from_secs(5)).await;
    }
}

#[derive(StructOpt, Debug)]
#[structopt()]
struct Opt {
    /// Silence all output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Verbose mode (-v, -vv, -vvv, etc)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,

    /// Enable linear interpolation instead of logarithmic interpolation
    #[structopt(short = "l", long = "linear")]
    linear_interpolation: bool,

    /// Path of config file
    #[structopt(
        short = "c",
        long = "config",
        default_value = "/etc/ksm-regulator.hjson"
    )]
    config_file: PathBuf,

    /// Path of `run` file, which is used to toggle KSM
    #[structopt(
        short = "r",
        long = "run-file",
        default_value = "/sys/kernel/mm/ksm/run"
    )]
    ksm_run_file: PathBuf,

    /// Path of `sleep_millisecs` file
    #[structopt(
        short = "s",
        long = "sleep-millisecs-file",
        default_value = "/sys/kernel/mm/ksm/sleep_millisecs"
    )]
    ksm_sleep_millisecs_file: PathBuf,
}

#[async_std::main]
async fn main() -> Result {
    let opt = Opt::from_args();

    stderrlog::new()
        .module(module_path!())
        .quiet(opt.quiet)
        .verbosity(opt.verbose + 1)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    let task = select(process(&opt).boxed(), ctrl_c().boxed()).await;
    match task {
        Either::Left((r, _)) => r,
        Either::Right(((), _)) => Ok(()),
    }
}
