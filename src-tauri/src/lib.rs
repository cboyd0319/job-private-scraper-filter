extern crate self as jobsentinel;

use jobsentinel_application as application;

mod bootstrap;
mod desktop;
mod ipc;
mod policy;

/// Start the JobSentinel desktop application.
pub fn run() {
    #[cfg(feature = "embedded-ml")]
    {
        std::env::set_var("HF_HUB_DISABLE_IMPLICIT_TOKEN", "1");
        std::env::set_var(
            "HF_XET_CACHE",
            desktop::get_data_dir().join("ml_models").join(".xet"),
        );
    }
    bootstrap::run();
}
