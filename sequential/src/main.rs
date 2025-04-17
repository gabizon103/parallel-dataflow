use lesson4;

fn main() {
    let opts: lesson4::Opts = argh::from_env();
    lesson4::run(opts);
}
