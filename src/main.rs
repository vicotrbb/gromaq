#![forbid(unsafe_code)]

fn main() {
    let backend = gromaq::native_gpu::NativeWgpuBackend;
    let app_launcher = gromaq::cli::RealNativeAppLauncher;
    let exit = gromaq::cli::run_with_backend_and_app(std::env::args(), &backend, &app_launcher);
    print!("{}", exit.stdout);
    eprint!("{}", exit.stderr);
    std::process::exit(exit.code);
}
