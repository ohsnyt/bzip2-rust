use super::options::{BzOpts, Verbosity};

/// Reports app progress as per Verbosity setting. 
/// Accepts both &str and String message.
pub fn report<S: AsRef<str> + std::fmt::Display>(o: &BzOpts, this_v: Verbosity, msg: S) {
    if o.verbosity >= this_v {
        println!("{}", msg)
    }
}
