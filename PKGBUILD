# Maintainer: Alik Aslanyan <cplusplus@gmail.com>

pkgname=ksm-regulator
pkgver=0.1.0.1850d66
pkgrel=1
pkgdesc="KSM Regulator - is a daemon to automatically manage KSM"
license=("GPL3")
depends=('systemd')
makedepends=('rust' 'cargo')
arch=(x86_64)
backup=('etc/ksm-regulator.hjson' )

pkgver() { 
  cd ../
  local_version=$(grep '^version =' Cargo.toml|head -n1|cut -d\" -f2)
  local_commit=$(git log --pretty=format:'%h' -n 1)
  echo "$local_version.$local_commit"
}


build() {
    # Uncomment to build with Xargo (will allow LTO with stdlib, 30% less executable size)
    # xargo build --release --locked --target x86_64-unknown-linux-gnu
    cargo build --release --locked --target x86_64-unknown-linux-gnu 
}

package() {
    cd ..

    usrdir="$pkgdir/usr"
    mkdir -p $usrdir

    configdir="$pkgdir/etc/"
    mkdir -p $configdir
    cp ./package/*.hjson $configdir

    systemddir="$usrdir/lib/systemd/system/"
    mkdir -p $systemddir
    cp ./package/*.service $systemddir/ksm-regulator.service

    install -Dm 755 target/x86_64-unknown-linux-gnu/release/${pkgname} -t "$usrdir/bin"
}
