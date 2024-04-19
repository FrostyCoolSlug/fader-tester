# GoXLR Fader Tester

This is a simple tool to test the GoXLRs ability to correctly hit volume targets under certain situations, the tests 
performed are as follows:

* Assigning a Channel to a Fader hits the correct volume (Within a 2% margin of Error)
* Setting the volume to 255 (100%) always and accurately hits the value (no margin for error)
* Setting the Volume to 0 (0%) always and accurately hits the value (no margin for error)
* Restoring the Volume from 0 hits the correct volume (Within a 2% margin of error)
* Muting / Unmuting a channel causes no movement in the faders

## Running
* Have Rust
* Close the Official App and the GoXLR Utility
* Run `cargo run`