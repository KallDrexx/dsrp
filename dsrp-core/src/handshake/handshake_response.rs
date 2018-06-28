use std::io;

pub enum HandshakeResponse {
    Success,
    Failure{reason: String},
}

impl HandshakeResponse {
    pub fn into_bytes(self) -> Vec<u8> {
        unimplemented!();
    }

    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, io::Error> {
        unimplemented!();
    }
}