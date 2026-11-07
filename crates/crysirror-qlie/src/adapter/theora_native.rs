//! theora methods of 6DF950 beyond load/play/pose.
//!
//! The movie object keeps its settings in TImoMovie fields: ss (+74/+78),
//! pp (+68..+70), ost (+9C..+A1), out (+8F), in (+8B), bm (+93), tf (+A6),
//! v (+82), vtn (+86). Queries answer the stored values; ?sk and sk use the
//! shared media clock, s stops playback (6A9AA8), tt equals ?tt.
use super::*;
use crysirror_core::service::MediaRequest;

impl QlieVm {
    /// Returns false when `op` is not handled here.
    pub(super) fn theora_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let stored = |vm: &Self, key: &str, i: usize, default: i32| -> Value {
            vm.objects[id]
                .properties
                .get(key)
                .and_then(|v| v.get(i))
                .cloned()
                .unwrap_or(Value::Int(default))
        };
        match op {
            "ss" | "pp" | "ost" => {
                let count = match op { "ss" => 2, _ => 3 };
                ensure!(a.len() == count, "QLIE movie setting ABI");
                let mut values = a.to_vec();
                if op == "ost" {
                    values[0] = Value::Int(i32::from(a[0].integer()? == 1));
                }
                self.objects.get_mut(id).unwrap().properties.insert(format!("movie/{op}"), values);
            }
            "?ss" | "?pp" | "?ost" => {
                let key = format!("movie/{}", &op[1..]);
                let count = if op == "?ss" { 2 } else { 3 };
                for i in 0..count {
                    m.result(i, stored(self, &key, i, 0));
                }
            }
            "?bm" | "?in" | "?out" | "?tf" | "?vtn" => {
                let default = match op { "?in" | "?out" => 0xFFFFFFF, _ => 0 };
                m.result(0, stored(self, &op[1..], 0, default));
            }
            "?v" => {
                let gain = self.objects[id].gain;
                m.result(0, Value::Int((gain * 256.).round() as i32));
            }
            "?l" => {
                let name = self.objects[id].asset.clone().unwrap_or_default();
                m.result_text(0, name);
            }
            "?sk" => {
                let position = match self.objects[id].media {
                    Some(media) => h.media_info(self.client, media)?.position_ms,
                    None => 0,
                };
                m.result(0, Value::Int(i32::try_from(position)?));
            }
            "sk" => {
                ensure!(a.len() == 1, "QLIE movie seek ABI");
                if let Some(media) = self.objects[id].media {
                    let position_ms = a[0].integer()?.max(0) as u64;
                    h.media_request(self.client, MediaRequest::Seek { id: media, position_ms })?;
                }
            }
            "s" => {
                if let Some(media) = self.objects[id].media {
                    h.media_request(self.client, MediaRequest::Stop { id: media })?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
