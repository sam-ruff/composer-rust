use clap::Args;

/// Test conditional output
#[derive(Args, Debug)]
#[command(hide = true)]
pub struct Test {
    text: String,
}

impl Test {
    pub fn exec(&self) -> anyhow::Result<()> {
        trace!("trace {}", self.text);
        debug!("debug {}", self.text);
        info!("info {}", self.text);
        success!("success {}", self.text);
        waiting!("waiting {}", self.text);
        warn!("warn {}", self.text);
        error!("error {}", self.text);
        display!("display {}", self.text);
        critical!("critical {}", self.text);
        Ok(())
    }
}
