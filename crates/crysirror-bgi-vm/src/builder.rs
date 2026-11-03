#[derive(Default)]
pub struct Builder {
    pub code: Vec<u8>,
    strings: Vec<(usize, String)>,
}
impl Builder {
    pub fn int(&mut self, value: i32) -> &mut Self {
        self.code.push(2);
        self.code.extend(value.to_le_bytes());
        self
    }
    pub fn string(&mut self, text: &str) -> &mut Self {
        let at = self.code.len();
        self.code.extend([5, 0, 0]);
        self.strings.push((at, text.into()));
        self
    }
    pub fn call(&mut self, group: u8, id: u8) -> &mut Self {
        self.code.extend([group, id]);
        self
    }
    pub fn ret(&mut self) -> &mut Self {
        self.code.push(0x17);
        self
    }
    pub fn finish(mut self) -> Vec<u8> {
        for (at, text) in self.strings {
            let displacement =
                i16::try_from(self.code.len() - at).expect("fixture string displacement");
            self.code[at + 1..at + 3].copy_from_slice(&displacement.to_le_bytes());
            let (encoded, _, bad) = encoding_rs::SHIFT_JIS.encode(&text);
            assert!(!bad, "fixture CP932 text");
            self.code.extend_from_slice(&encoded);
            self.code.push(0);
        }
        let mut image = vec![0; 16];
        image[..4].copy_from_slice(&16u32.to_le_bytes());
        image[4..8].copy_from_slice(&(self.code.len() as u32).to_le_bytes());
        image.extend(self.code);
        image
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Machine, Step};
    #[test]
    fn pending_native_call_is_not_acknowledged_on_host_failure() {
        let mut b = Builder::default();
        b.int(7).string("sound.ogg").int(128).call(0xa0, 0x10).ret();
        let mut m = Machine::from_bp(&b.finish()).unwrap();
        let c = loop {
            match m.step().unwrap() {
                Step::Continue => {}
                Step::Call(call) => break call,
                step => panic!("unexpected {step:?}"),
            }
        };
        let before = m.pc();
        assert_eq!(c.args[0], 7);
        assert_eq!(m.string(c.args[1]).unwrap(), "sound.ogg");
        assert_eq!(c.args[2], 128);
        assert_eq!(m.step().unwrap(), Step::Call(c));
        assert!(m.commit_call(&[1]).is_err());
        assert_eq!(m.pc(), before);
        m.commit_call(&[]).unwrap();
        assert_eq!(m.step().unwrap(), Step::Halted);
    }
    #[test]
    fn wrapping_arithmetic_and_native_operand_ring_are_preserved() {
        let mut b = Builder::default();
        b.int(i32::MAX).int(1);
        b.code.push(0x20);
        b.ret();
        let mut m = Machine::from_bp(&b.finish()).unwrap();
        m.step().unwrap();
        m.step().unwrap();
        m.step().unwrap();
        assert_eq!(m.top(), i32::MIN);
        for n in 0..4097 {
            m.push(n);
        }
        assert_eq!(m.stack_index(), 2);
        assert_eq!(m.pop(), 4096);
        assert_eq!(m.pop(), 4095);
    }
    #[test]
    fn malformed_images_and_unknown_instructions_fail_visibly() {
        assert!(Machine::from_bp(&[0; 16]).is_err());
        let mut b = Builder::default();
        b.code.push(0x03);
        let mut m = Machine::from_bp(&b.finish()).unwrap();
        assert!(m.step().unwrap_err().to_string().contains("BGI"));
        assert_eq!(m.pc(), 16);
    }
}
