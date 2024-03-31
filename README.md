# OpenPID Rust Codegen
This repository generates Rust code (relying on embedded-hal abstractions) given a peripheral's openPID description.

## Scope
Any Rust code, though support should be through embedded-hal in nearly every case. 

Only if a platform cannot support embedded-hal should non-embeded-hal code be generated from this repository. 
