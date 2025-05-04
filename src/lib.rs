pub mod blockchain {
    pub mod block;
    pub mod processor;
    pub mod state;
    pub mod transaction;
    pub mod validator;
}
pub mod constants;
pub mod helpers {
    pub mod args;
    pub mod io;
    pub mod serialization;
}
pub mod poh {
    pub mod core;
    pub mod thread;
    pub mod verifier;
}

#[cfg(test)]
mod tests;
