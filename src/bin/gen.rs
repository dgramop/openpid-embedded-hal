use openpid::prelude::*;
use openpid::OpenPID;
pub fn main() {
    openpid_embedded_hal::RustEmbeddedHal::new(&OpenPID::from_str("/tmp/openpid.toml").expect("Couldn't parse openPID"), "/tmp/out").codegen().expect("Codegen Failed");
}
