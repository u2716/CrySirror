// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original default user commands (MUGEN2 535980 ReceiveUserCommand0).
//!
//! The legacy constructor (542180) walks the fixed user-command list
//! (515630/5155c0/515590: selectplace 22, selectheroin 31, printmovie 27,
//! printcalendar 30, printresult 33, selectchart 34, nameinput 29,
//! selectplace2 35, selectzukan 24) and registers a command only when the
//! game setup has a nonzero `<name>IsDefault`; registration looks the name up
//! in `spt/command.xtx` (index / 2). Registered commands clear the four skip
//! words, capture the exit screen and switch to their mode class; others go to
//! the game's own ReceiveUserCommand (vtable+32), which is not reviewed here.
//!
//! The retained native receipt is acknowledged only after the mode actor
//! finishes (PrintMovie Calcu 4fe010 calls SetYoyaku), so the VM PC never
//! advances while the mode owns the frame.
use super::*;

const DEFAULT_COMMANDS: &[(i32, &[u8])] = &[
    (22, b"selectplace"),
    (31, b"selectheroin"),
    (27, b"printmovie"),
    (30, b"printcalendar"),
    (33, b"printresult"),
    (34, b"selectchart"),
    (29, b"nameinput"),
    (35, b"selectplace2"),
    (24, b"selectzukan"),
];

/// Registered default command code -> original mode class.
pub(crate) fn default_user_commands(
    configuration: &crate::native_bootstrap::NativeConfiguration,
) -> Result<BTreeMap<u32, i32>> {
    let mut table = BTreeMap::new();
    for (mode, name) in DEFAULT_COMMANDS {
        let key = [*name, b"IsDefault".as_slice()].concat();
        if configuration.setup.integer(&key)?.unwrap_or(0) == 0 {
            continue;
        }
        // CNameList::SearchName returns the first matching token; odd hits
        // are display names and still divide down to their pair index.
        if let Some(index) = configuration.commands.index(name) {
            table.entry(u32::from(index) / 2).or_insert(*mode);
        }
    }
    Ok(table)
}

pub(super) struct UserModeWait {
    call: NativeCall,
    mode: i32,
    number: i32,
    requested: bool,
    finished: bool,
}

/// Mode classes with an engine-side actor for the retained command.
const ACTOR_MODES: &[i32] = &[22, 27];

impl Bridge {
    pub(crate) fn configure_user_commands(
        &mut self,
        configuration: &crate::native_bootstrap::NativeConfiguration,
    ) -> Result<()> {
        if self.user_commands.is_none() {
            self.user_commands = Some(Arc::new(default_user_commands(configuration)?));
        }
        Ok(())
    }
    /// Some(complete) when this receipt is an original default user command.
    /// Kept out of line: `try_commit` sits on deep owner stacks.
    #[inline(never)]
    pub(super) fn try_user_command(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
    ) -> Result<Option<bool>> {
        let Payload::UserCall {
            command: true,
            parameters,
            ..
        } = &call.payload
        else {
            return Ok(None);
        };
        if let Some(wait) = self.user_mode.as_ref() {
            ensure!(
                &wait.call == call,
                "NNN user-command mode {} owns another native receipt",
                wait.mode
            );
            if !wait.finished {
                return Ok(Some(false));
            }
            machine.acknowledge(call)?;
            self.user_mode = None;
            return Ok(Some(true));
        }
        let mode = self
            .user_commands
            .as_ref()
            .and_then(|table| table.get(&call.code).copied());
        match mode {
            Some(mode) if ACTOR_MODES.contains(&mode) => {
                // 535980 clears the same five words as stopf5. Only
                // PrintMovie's StartUserCommand (4fdfb0) reads a parameter
                // (movie number, default 1); SelectPlace has the base no-op.
                self.skip.stop_f5();
                self.user_mode = Some(Box::new(UserModeWait {
                    call: call.clone(),
                    mode,
                    number: parameters.first().copied().unwrap_or(1),
                    requested: true,
                    finished: false,
                }));
                Ok(Some(false))
            }
            Some(mode) => bail!(
                "System-NNN default user command code {} requires native mode {mode} actor",
                call.code
            ),
            None => bail!(
                "System-NNN user command code {} is handled by the game's own ReceiveUserCommand override (not reviewed)",
                call.code
            ),
        }
    }
    /// One-shot request for the installed owner: (mode class, movie number).
    pub(crate) fn take_user_command_mode(&mut self) -> Option<(i32, i32)> {
        let wait = self.user_mode.as_mut()?;
        if !wait.requested {
            return None;
        }
        wait.requested = false;
        Some((wait.mode, wait.number))
    }
    /// Mode class owning the retained command until its SetYoyaku.
    pub(crate) fn user_command_mode(&self) -> Option<i32> {
        self.user_mode
            .as_ref()
            .filter(|wait| !wait.finished)
            .map(|wait| wait.mode)
    }
    /// The mode's exit called SetYoyaku. The next VM step acknowledges.
    pub(crate) fn finish_user_command_mode(&mut self) -> Result<()> {
        let wait = self
            .user_mode
            .as_mut()
            .context("NNN user-command mode return without its command")?;
        ensure!(!wait.requested, "NNN user-command mode finished before it started");
        wait.finished = true;
        Ok(())
    }
}
