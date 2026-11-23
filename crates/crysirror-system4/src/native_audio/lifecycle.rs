// Native resource/command lifetime; GPL-2.0-or-later, provenance in parent.
// No decoder, fader or mixer kernel. Every transition consumes a scoped engine
// receipt. Tickets/resources remain owned here until each cleanup succeeds.
use super::*;
impl NativeAudio {
    fn parameters(key: Key, channel: &Channel) -> Result<AudioParameters> {
        ensure!(
            channel.loop_start < channel.loop_end,
            "System4 native audio has unsupported empty loop range"
        );
        Ok(AudioParameters {
            gain: f32::from(channel.volume) / 100.,
            looping: true,
            loop_passes: if channel.count == 0 {
                None
            } else {
                Some(channel.count)
            },
            swap_channels: channel.swapped,
            loop_start: channel.loop_start,
            loop_end: Some(channel.loop_end),
            start_frame: Some(0),
            paused: false,
            retain_on_finish: true,
            transport_active: false,
            mix_group: Some(Self::group_name(channel.group)),
            bus: if key.kind == Kind::Music {
                PcmBus::Music
            } else {
                PcmBus::Sfx
            },
            ..Default::default()
        })
    }
    pub(super) fn poll_command(
        &mut self,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let Some(pending) = self.pending.as_ref() else {
            return Ok(None);
        };
        let original = match pending {
            Pending::Locate { call, .. }
            | Pending::Prepare { call, .. }
            | Pending::Warm { call, .. }
            | Pending::Transport { call, .. } => call,
        };
        ensure!(original == call, "System4 stale canonical audio receipt");
        let owner = self.owner;
        if let Some(Pending::Locate {
            key,
            task,
            resolved,
            ..
        }) = self.pending.as_mut()
        {
            let key = *key;
            if resolved.is_none() {
                let Some(path) = task.try_take()? else {
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                };
                *resolved = Some(path);
            }
            let Some(path) = resolved.as_ref().unwrap().clone() else {
                self.retire(key, host)?;
                self.pending = None;
                machine.complete_hll(call, HllReturn::Scalar(0))?;
                return Ok(Some(Dispatch::Completed));
            };
            let ticket = host.begin_audio_resource(
                owner,
                AudioAsset {
                    path,
                    frame_limit: None,
                },
                true,
            )?;
            self.pending = Some(Pending::Prepare {
                call: call.clone(),
                key,
                data_number: -1,
                ticket: Some(ticket),
                resource: None,
            });
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        }
        let pending = self.pending.as_ref().unwrap();
        if let Pending::Prepare {
            key,
            data_number,
            ticket,
            ..
        } = pending
        {
            let (key, number, ticket) = (*key, *data_number, *ticket);
            let ticket = ticket.context("System4 resource receipt already released")?;
            let status = host.audio_resource_preparation(owner, ticket)?;
            match status.phase {
                AudioPreparePhase::Pending => {
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
                }
                AudioPreparePhase::Failed if key.kind==Kind::Music => return self.failed_music(machine,call,key,status.error.as_deref().unwrap_or("missing failure"),bridge,host).map(Some),
                AudioPreparePhase::Failed => anyhow::bail!("System4 shared audio resource: {}",status.error.as_deref().unwrap_or("missing failure")),
                AudioPreparePhase::Ready => {}
            }
            let resource = status
                .resource
                .context("System4 prepared audio resource absent")?;
            ensure!(
                resource.client == owner.0,
                "System4 foreign prepared audio resource"
            );
            if let Some(Pending::Prepare { resource: held, .. }) = self.pending.as_mut() {
                *held = Some(resource);
            }
            let channel = self.new_channel(
                key,
                number,
                resource,
                status
                    .info
                    .context("System4 prepared audio metadata absent")?,
            )?;
            // Prewarm a canonical source while silent. This is not a reopen on
            // every Play; the same scoped channel remains allocated until the
            // explicit native Unprepare/close transition.
            let voice = host.begin_audio(
                owner,
                AudioRequest::PlayResource {
                    channel: key.channel()?,
                    resource,
                    parameters: Self::parameters(key, &channel)?,
                },
            )?;
            self.pending = Some(Pending::Warm {
                call: call.clone(),
                key,
                resource_ticket: Some(ticket),
                voice_ticket: Some(voice),
                resource: Some(resource),
                channel: Some(channel),
                ready: false,
                stopped: false,
            });
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        }
        if let Some(Pending::Warm {
            key,
            voice_ticket,
            ready,
            ..
        }) = self.pending.as_ref()
        {
            let key = *key;
            if !*ready {
                let ticket = voice_ticket.context("System4 canonical voice receipt absent")?;
                let status = host.audio_preparation(owner, ticket)?;
                ensure!(
                    status.channel == key.channel()?,
                    "System4 foreign canonical voice receipt"
                );
                match status.phase {
                    AudioPreparePhase::Pending => {
                        return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
                    }
                    AudioPreparePhase::Failed if key.kind==Kind::Music => return self.failed_music(machine,call,key,status.error.as_deref().unwrap_or("missing failure"),bridge,host).map(Some),
                    AudioPreparePhase::Failed => anyhow::bail!("System4 canonical source preparation: {}",status.error.as_deref().unwrap_or("missing failure")),
                    AudioPreparePhase::Ready => {}
                }
                if let Some(Pending::Warm { ready, .. }) = self.pending.as_mut() {
                    *ready = true;
                }
            }
            // Each successful release clears its authority before a later
            // fallible operation. Exit/retry can never lose or double-release it.
            if let Some(Pending::Warm { voice_ticket, .. }) = self.pending.as_mut() {
                if let Some(ticket) = *voice_ticket {
                    host.release_audio(owner, ticket)?;
                    *voice_ticket = None;
                }
            }
            if let Some(Pending::Warm {
                resource_ticket, ..
            }) = self.pending.as_mut()
            {
                if let Some(ticket) = *resource_ticket {
                    host.release_audio_resource_preparation(owner, ticket)?;
                    *resource_ticket = None;
                }
            }
            if let Some(old) = self.channels.get(&key) {
                host.release_audio_resource(owner, old.resource)?;
                self.channels.remove(&key);
            }
            let Some(Pending::Warm {
                channel, resource, ..
            }) = self.pending.as_mut()
            else {
                unreachable!()
            };
            let channel = channel
                .take()
                .context("System4 canonical source state lost")?;
            *resource = None;
            self.channels.insert(key, channel);
            self.pending = None;
            machine.complete_hll(call, HllReturn::Scalar(1))?;
            return Ok(Some(Dispatch::Completed));
        }
        let Some(Pending::Transport {
            key, ticket, ready, ..
        }) = self.pending.as_ref()
        else {
            unreachable!()
        };
        let key = *key;
        if !*ready {
            let status = host.audio_seek_preparation(
                owner,
                ticket.context("System4 transport seek receipt absent")?,
            )?;
            ensure!(
                status.channel == key.channel()?,
                "System4 foreign canonical seek receipt"
            );
            match status.phase {
                AudioPreparePhase::Pending => {
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
                }
                AudioPreparePhase::Failed => anyhow::bail!(
                    "System4 canonical seek: {}",
                    status.error.as_deref().unwrap_or("missing failure")
                ),
                AudioPreparePhase::Ready => {}
            }
            if let Some(Pending::Transport { ready, .. }) = self.pending.as_mut() {
                *ready = true;
            }
        }
        if let Some(Pending::Transport { ticket, .. }) = self.pending.as_mut() {
            if let Some(id) = *ticket {
                host.release_audio_seek(owner, id)?;
                *ticket = None;
            }
        }
        let Some(Pending::Transport {
            playing,
            reported_position,
            ..
        }) = self.pending.as_ref()
        else {
            unreachable!()
        };
        let (playing, reported_position) = (*playing, *reported_position);
        host.submit_audio(
            owner,
            AudioRequest::SetTransportActive {
                channel: key.channel()?,
                active: playing,
            },
        )?;
        let channel = self
            .channels
            .get_mut(&key)
            .context("System4 canonical source state lost")?;
        channel.playing = playing;
        channel.hold_position = reported_position.is_some();
        channel.position = reported_position.unwrap_or(0);
        self.pending = None;
        machine.complete_hll(call, HllReturn::Scalar(1))?;
        Ok(Some(Dispatch::Completed))
    }
    pub(super) fn transport(
        &mut self,
        key: Key,
        playing: bool,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Dispatch> {
        let channel = self
            .channels
            .get(&key)
            .context("System4 canonical source absent")?;
        let old_position = channel.position;
        if playing {
            host.submit_audio(
                self.owner,
                AudioRequest::SetLoopSchedule {
                    channel: key.channel()?,
                    start: channel.loop_start,
                    end: Some(channel.loop_end),
                    passes: if channel.count == 0 {
                        None
                    } else {
                        Some(channel.count)
                    },
                },
            )?;
            host.submit_audio(
                self.owner,
                AudioRequest::SetChannelSwap {
                    channel: key.channel()?,
                    swapped: channel.swapped,
                },
            )?;
        }
        let ticket = host.begin_audio_seek(self.owner, key.channel()?, 0)?;
        // Quiesce immediately, before another frame can consume the new cursor.
        // The seek receipt remains authoritative across asynchronous waits.
        self.pending = Some(Pending::Transport {
            call: call.clone(),
            key,
            ticket: Some(ticket),
            ready: false,
            playing,
            reported_position: if !playing && key.kind == Kind::Music {
                Some(old_position)
            } else {
                None
            },
        });
        host.submit_audio(
            self.owner,
            AudioRequest::SetTransportActive {
                channel: key.channel()?,
                active: false,
            },
        )?;
        Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)))
    }
    pub(super) fn prepare_file(
        &mut self,
        machine: &mut Machine,
        call: &ExternalCall,
        args: &[Argument],
        host: &mut dyn HostServices,
    ) -> Result<Dispatch> {
        let [Argument::Scalar { kind: 10, bits: id }, Argument::String { bytes, .. }] = args else {
            anyhow::bail!("System4 Sound_PrepareFromFile original ABI");
        };
        if *id < 0 {
            machine.complete_hll(call, HllReturn::Scalar(0))?;
            return Ok(Dispatch::Completed);
        }
        let key = Key {
            kind: Kind::Sound,
            number: *id,
        };
        key.channel()?;
        let bytes = bytes.split(|b| *b == 0).next().unwrap();
        ensure!(
            !bytes.is_empty() && bytes.len() <= 4096,
            "System4 audio filename bounds"
        );
        let name =
            crysirror_resource::text_stream::decode_shift_jis(bytes, 4096)?.replace('\\', "/");
        let requested = VfsPath::new(Mount::Asset, &name)?;
        let storage = host.storage(self.owner)?;
        let task = self._pool.spawn(move |cancel| {
            cancel.check()?;
            let found = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                storage.as_ref(),
                &requested,
            )?;
            cancel.check()?;
            Ok(found)
        })?;
        self.pending = Some(Pending::Locate {
            call: call.clone(),
            key,
            task,
            resolved: None,
        });
        Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1)))
    }
    pub(super) fn close_commands(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(pending) = self.pending.as_mut() {
            match pending {
                Pending::Locate { .. } => {}
                Pending::Prepare {
                    ticket, resource, ..
                } => {
                    if let Some(handle) = *resource {
                        host.release_audio_resource(self.owner, handle)?;
                        *resource = None;
                    }
                    if let Some(id) = *ticket {
                        host.release_audio_resource_preparation(self.owner, id)?;
                        *ticket = None;
                    }
                }
                Pending::Warm {
                    key,
                    resource_ticket,
                    voice_ticket,
                    resource,
                    stopped,
                    ..
                } => {
                    if let Some(id) = *voice_ticket {
                        host.release_audio(self.owner, id)?;
                        *voice_ticket = None;
                    }
                    if !*stopped {
                        host.submit_audio(
                            self.owner,
                            AudioRequest::Stop {
                                channel: key.channel()?,
                                fade_ms: 0.,
                            },
                        )?;
                        *stopped = true;
                    }
                    if let Some(handle) = *resource {
                        host.release_audio_resource(self.owner, handle)?;
                        *resource = None;
                    }
                    if let Some(id) = *resource_ticket {
                        host.release_audio_resource_preparation(self.owner, id)?;
                        *resource_ticket = None;
                    }
                }
                Pending::Transport { ticket, .. } => {
                    if let Some(id) = *ticket {
                        host.release_audio_seek(self.owner, id)?;
                        *ticket = None;
                    }
                }
            }
            self.pending = None;
        }
        Ok(())
    }
}
