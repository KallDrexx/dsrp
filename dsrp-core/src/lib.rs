#[macro_use] extern crate failure;
extern crate byteorder;

#[cfg(test)]
#[macro_use]
mod test_utils {
    #[macro_use] pub mod assert_vec_contains_macro;
}

pub mod handshake;
pub mod messages;
pub mod server_handler;
pub mod client_handler;
