use path_to_unicode_filename::*;

fn main() -> Result<(), Error> {
    let mut args = std::env::args();
    args.next();
    for arg in args {
        println!("{}", to_filename(arg)?);
    }
    Ok(())
}

