pub struct CommandAction {
    pub command: String,

    pub start: PathWrapper,
    pub shell: String,

    pub stdout: bool,
    pub stderr: bool,

    pub clean_env: bool,
    pub env: EnvMap,

    pub nonzero_exit: NonZeroExitBehavior,
}

impl Resolve for CommandAction {
    #[inline]
    fn resolve<C>(self, opts: &ResolveOpts, cache: &mut C) -> ResolveResult
    where
        C: Cache,
    {
        let Self {
            command,
            start,
            shell,
            stdout,
            stderr,
            clean_env,
            env,
            nonzero_exit,
        } = self;

        sl_debug!("Building command...");

        let mut cmd = Command::new(shell);
        cmd.args(&["-c", &command]).current_dir(&start.abs());

        if !stdout {
            sl_debug!("Capturing stdout...");
            cmd.stdout(Stdio::null());
        }
        if !stderr {
            sl_debug!("Capturing stderr...");
            cmd.stderr(Stdio::null());
        }

        if clean_env {
            sl_debug!("Clearing environment variables...");
            cmd.env_clear();
        }

        if !env.is_empty() {
            sl_debug!("Populating environment variables...");
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        sl_debug!("Spawning...");
        let mut child = fail!(cmd.spawn(), err => {
            sl_error!("{$red}Couldn't spawn command:{/$} {}", err);
        });

        let res = fail!(child.wait(), err => {
            sl_error!("{$red}Couldn't finish command:{/$} {}", err);
        });

        if let Some(code) = res.code() {
            sl_debug!("Done... exit {[green]}", code);
        }

        // Check for non zero exit status.
        if !res.success() {
            match nonzero_exit {
                NonZeroExitBehavior::Error => {
                    sl_error!(
                        "{$red}Hook{/$} '{[dimmed]}' {$red}exited with a non-zero status{/$}",
                        command
                    );
                    return Err(EmptyError);
                }
                NonZeroExitBehavior::Warn => sl_warn!(
                    "{$yellow}Hook{/$} '{[dimmed]}' {$yellow}exited with a non-zero status{/$}",
                    command,
                ),
                NonZeroExitBehavior::Ignore => {}
            };
        }

        Ok(Resolution::Done)
    }
}
