//! An application that keeps one window per user.
//!
//! Start it twice. The second start hands its arguments and working
//! directory to the first and exits at once; the first prints them and comes
//! to the front.

use std::time::Duration;

use bones_kernel::bus::{Envelope, Handler, Module, ModuleContext};
use bones_kernel::instance::{claim, Claim, Handoff};
use bones_kernel::messages::instance::Launched;
use bones_kernel::messages::{DecodeMessage, Message};
use bones_kernel::Kernel;

/// The `app_uid` every launch of this application claims, which keeps it
/// apart from every other application built on bones. A real application
/// passes its configured executable stem.
const APP_UID: &str = "bones-single-instance-demo";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The claim comes first, before the engine opens anything: a second
    // instance that had already opened a window would be too late.
    let handoff = Handoff {
        args: std::env::args().skip(1).collect(),
        cwd: std::env::current_dir()?,
    };
    match claim(APP_UID, handoff, Duration::from_secs(2))? {
        Claim::HandedOff => {
            println!("handed off to the running instance");
            Ok(())
        }
        Claim::Primary(primary) => {
            println!("this is the instance; start the app again to hand it arguments");
            Kernel::new()
                .window("bones single-instance demo", 640, 360)
                .renderer()
                .module(LaunchPrinter)
                .single_instance(primary)
                .run()?;
            Ok(())
        }
    }
}

/// Stands in for whatever an application does with a later launch: open the
/// file, switch to the repository, show the view the arguments name.
struct LaunchPrinter;

impl Handler for LaunchPrinter {
    fn handle(&mut self, envelope: &Envelope) {
        if let Ok(launched) = Launched::decode(&envelope.payload) {
            println!("launched again from {}: {:?}", launched.cwd, launched.args);
        }
    }
}

impl Module for LaunchPrinter {
    fn name(&self) -> &str {
        "launch-printer"
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        context.subscribe(Launched::TOPIC);
        Ok(())
    }
}
