# Maintainer: Alik Aslanyan <cplusplus@gmail.com>

pkgname=ksm-regulator
pkgver=0.1.0.r0.g758ceb0
pkgrel=1
pkgdesc="KSM Regulator - is a daemon to automatically manage KSM"
license=("GPL3")
depends=(systemd)
makedepends=(rust)
arch=(x86_64)
backup=( 'etc/ksm-regulator.hjson' )

pkgver() {
    (git describe --long --tags || echo "$pkgver") | sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
}

build() {
    cargo build --release --locked
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

    install -Dm 755 target/release/${pkgname} -t "$usrdir/bin"
}
