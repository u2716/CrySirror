// Copyright (c) 2026 CrySirror contributors.
// Native semantics researched against MajiroTools (C) 2021 Mario Welzig,
// Robert Jordan and its pinned opcode wiki. MIT; see SOURCE.json.
//! Bounded native stack execution. External calls remain unacknowledged at
//! their original PC until the host operation actually succeeds.
use crate::program::{Instruction, Operand, Program};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
mod scalar;
mod modules;
mod text;
mod events;
mod threads;
pub(crate) mod arrays;
pub mod save;

const MAX_STACK: usize = 65536;
const MAX_LOCALS: usize = 16384;
const MAX_FRAMES: usize = 128;
#[derive(Clone, Debug)]
pub enum Value {
    Integer(i32),
    Float(f32),
    String(Vec<u8>),
    /// Native refcounted array object (401D60/41D980): shared by reference.
    Array(ArrayRef),
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => Arc::ptr_eq(&a.0, &b.0),
            _ => false,
        }
    }
}
/// Native array header: element kind and dimensions [d8,d12,d16] where d16
/// is the innermost (last subscript). Indices wrap with a positive modulo.
#[derive(Debug)]
pub struct ArrayData {
    pub element: u8,
    pub dims: [i32; 3],
    pub elements: Vec<Value>,
}
#[derive(Clone, Debug)]
pub struct ArrayRef(pub Arc<std::sync::Mutex<ArrayData>>);
impl ArrayRef {
    pub fn new(element: u8, dims: [i32; 3]) -> Result<Self> {
        ensure!(element <= 2, "Majiro array element type");
        let count = dims
            .iter()
            .try_fold(1usize, |n, d| n.checked_mul((*d).max(1) as usize))
            .context("Majiro array extent")?;
        ensure!(count <= 4 * 1024 * 1024, "Majiro array element budget");
        let fill = Value::default_type(element)?;
        Ok(Self(Arc::new(std::sync::Mutex::new(ArrayData {
            element,
            dims: dims.map(|d| d.max(1)),
            elements: vec![fill; count],
        }))))
    }
    pub fn lock(&self) -> std::sync::MutexGuard<'_, ArrayData> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
impl Value {
    /// Native41DC40 stores strlen+1 bytes, including the terminator. This is
    /// distinct from an encoded instruction's explicit byte extent.
    pub(crate) fn native_string(bytes:&[u8])->Result<Self> {
        let bytes=&bytes[..bytes.iter().position(|byte|*byte==0).unwrap_or(bytes.len())];
        ensure!(bytes.len()<1024*1024,"Majiro native string byte budget");
        let mut value=Vec::with_capacity(bytes.len()+1);
        value.extend_from_slice(bytes);value.push(0);
        Ok(Self::String(value))
    }
    pub fn integer(&self) -> Result<i32> {
        if let Self::Integer(n) = self {
            Ok(*n)
        } else {
            bail!("Majiro integer operand required")
        }
    }
    /// C-string view (up to the first NUL), as native char* consumers see it.
    pub fn bytes(&self) -> Result<&[u8]> {
        if let Self::String(b) = self {
            Ok(&b[..b.iter().position(|x| *x == 0).unwrap_or(b.len())])
        } else {
            bail!("Majiro string operand required")
        }
    }
    /// Stored allocation including its terminator (40DA90 concatenation).
    pub fn stored(&self) -> Result<&[u8]> {
        if let Self::String(b) = self {
            Ok(b)
        } else {
            bail!("Majiro string operand required")
        }
    }
    fn default_type(t: u8) -> Result<Self> {
        match t {
            0 => Ok(Self::Integer(0)),
            1 => Ok(Self::Float(0.)),
            //41D980 creates a one-byte string allocation containing NUL.
            2 => Ok(Self::String(vec![0])),
            //41D980 types3..5: a 1x1x1 array holding one default element.
            3..=5 => Ok(Self::Array(ArrayRef::new(t - 3, [1, 1, 1])?)),
            _ => bail!("Majiro variable type"),
        }
    }
    pub(crate) fn kind(&self) -> u8 {
        match self {
            Self::Integer(_) => 0,
            Self::Float(_) => 1,
            Self::String(_) => 2,
            Self::Array(a) => 3 + a.lock().element,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Arguments are in native parameter order (stack top first).
    Native {
        pc: u32,
        hash: u32,
        arguments: Vec<Value>,
        returns: bool,
    },
    Text {
        pc: u32,
        bytes: Vec<u8>,
    },
    ProcessText {
        pc: u32,
    },
    Control {
        pc: u32,
        bytes: Vec<u8>,
    },
    Halted,
}
#[derive(Clone)]
struct Frame {
    serial: u64,
    interrupted: Option<(Event,u32,usize)>,
    global_initializer: bool,
    return_pc: u32,
    return_module: usize,
    stack_base: usize,
    arguments: Vec<Value>,
    locals: Vec<Value>,
    discard_return: bool,
    bsel: Bsel,
}
/// Per-frame click/AIP targets (frame offsets 20..36 in 40B350). Zero = unset.
#[derive(Clone, Copy, Default)]
pub(crate) struct Bsel {
    click: [u32; 3],
    aip4: u32,
    destructor: u32,
}
pub struct Machine {
    pub program: Arc<Program>,
    pub pc: u32,
    pub line: u16,
    pub executed: u64,
    pub stack: Vec<Value>,
    frames: Vec<Frame>,
    banks: [BTreeMap<u32, Value>; 3],
    pending: Option<(Event, u32, usize)>,
    halted: bool,
    active_module: usize,
    modules: Vec<modules::LinkedModule>,
    call_cache: BTreeMap<(usize,u32),(usize,u32)>,
    hooks: Vec<events::Hook>,
    scheduler: threads::Scheduler,
    next_frame: u64,
    clicks: u8,
    initializer_finished: bool,
}
impl Machine {
    pub fn new(program: Arc<Program>) -> Self {
        Self {
            pc: program.entry,
            modules: vec![modules::LinkedModule {name:"<entry>".into(),program:program.clone()}],
            active_module:0,
            call_cache:BTreeMap::new(),
            hooks:vec![],scheduler:threads::Scheduler::default(),next_frame:2,
            program,
            line: 0,
            executed: 0,
            stack: vec![],
            frames: vec![Frame {
                serial:1,interrupted:None,global_initializer:false,
                return_pc: 0,
                return_module: 0,
                stack_base: 0,
                arguments: vec![],
                locals: vec![],
                discard_return: false,
                bsel: Bsel::default(),
            }],
            banks: std::array::from_fn(|_| BTreeMap::new()),
            pending: None,
            halted: false,
            clicks: 0,
            initializer_finished: false,
        }
    }
    pub fn pending(&self) -> Option<&Event> {
        self.pending.as_ref().map(|p| &p.0)
    }
    pub fn acknowledge(&mut self, outputs: Vec<Value>) -> Result<()> {
        let (event, next, consumed) = self
            .pending
            .as_ref()
            .context("no pending Majiro operation")?;
        let expected = matches!(event, Event::Native { returns: true, .. }) as usize;
        ensure!(outputs.len() == expected, "Majiro native return count");
        let keep = self
            .stack
            .len()
            .checked_sub(*consumed)
            .context("Majiro native stack receipt")?;
        ensure!(
            keep + outputs.len() <= MAX_STACK,
            "Majiro return stack budget"
        );
        let next = *next;
        self.stack.truncate(keep);
        self.stack.extend(outputs);
        self.pc = next;
        self.executed += 1;
        self.pending = None;
        Ok(())
    }
    fn queue(&mut self, event: Event, next: u32, consumed: usize) -> Event {
        self.pending = Some((event.clone(), next, consumed));
        event
    }
    fn tail(&self, n: usize) -> Result<&[Value]> {
        let start = self
            .stack
            .len()
            .checked_sub(n)
            .context("Majiro stack underflow")?;
        ensure!(
            start >= self.frames.last().unwrap().stack_base,
            "Majiro operand crosses call frame"
        );
        Ok(&self.stack[start..])
    }
    fn push(&mut self, v: Value) -> Result<()> {
        ensure!(self.stack.len() < MAX_STACK, "Majiro stack budget");
        self.stack.push(v);
        Ok(())
    }
    fn local_index(&self, offset: i16) -> Result<usize> {
        ensure!(offset >= 0, "Majiro destination local offset");
        let n = offset as usize;
        ensure!(
            n < self.frames.last().unwrap().locals.len(),
            "Majiro local outside allocation"
        );
        Ok(n)
    }
    fn variable(&self, flags: u16, hash: u32, offset: i16) -> Result<Value> {
        self.variable_typed(flags, hash, offset, ((flags >> 8) & 7) as u8)
    }
    fn variable_typed(&self, flags: u16, hash: u32, offset: i16, ty: u8) -> Result<Value> {
        let scope = (flags >> 5) & 7;
        ensure!(
            flags & 0xe000 == 0 && ty <= 5,
            "unsupported Majiro variable flags/type"
        );
        if scope == 3 {
            let f = self.frames.last().unwrap();
            if offset == -1 {
                return Ok(Value::Integer(f.arguments.len() as i32));
            }
            let v = if offset >= 0 {
                f.locals.get(offset as usize)
            } else {
                let n = (-2i32 - offset as i32) as usize;
                f.arguments
                    .len()
                    .checked_sub(n + 1)
                    .and_then(|i| f.arguments.get(i))
            }
            .context("Majiro local/argument offset")?;
            ensure!(v.kind() == ty, "Majiro variable type mismatch");
            Ok(v.clone())
        } else {
            ensure!(scope < 3, "Majiro scope");
            let v = self.banks[scope as usize]
                .get(&hash)
                .cloned()
                .unwrap_or(Value::default_type(ty)?);
            ensure!(v.kind() == ty, "Majiro variable type mismatch");
            Ok(v)
        }
    }
    fn store(&mut self, flags: u16, hash: u32, offset: i16, value: Value) -> Result<()> {
        let scope = (flags >> 5) & 7;
        ensure!(scope <= 3, "Majiro scope");
        if scope == 3 && offset <= -2 {
            // Arguments are writable stack slots (offset -2 is the first).
            let f = self.frames.last_mut().unwrap();
            let n = (-2i32 - offset as i32) as usize;
            let i = f.arguments.len().checked_sub(n + 1).context("Majiro argument store offset")?;
            f.arguments[i] = value;
        } else if scope == 3 {
            let i = self.local_index(offset)?;
            ensure!(
                self.frames.last().unwrap().locals[i].kind() == value.kind(),
                "Majiro store type mismatch"
            );
            self.frames.last_mut().unwrap().locals[i] = value;
        } else {
            ensure!(
                self.banks[scope as usize].len() < 65536
                    || self.banks[scope as usize].contains_key(&hash),
                "Majiro variable budget"
            );
            self.banks[scope as usize].insert(hash, value);
        }
        Ok(())
    }
    pub fn step(&mut self) -> Result<Option<Event>> {
        if let Some(event) = self.pending() {
            return Ok(Some(event.clone()));
        }
        if self.halted {
            return Ok(Some(Event::Halted));
        }
        let i = self.program.instruction(self.pc)?.clone();
        self.execute(&i).with_context(|| {
            format!(
                "Majiro pc={:#x} line={} opcode={:#x}",
                self.pc, self.line, i.opcode
            )
        })
    }
    fn execute(&mut self, i: &Instruction) -> Result<Option<Event>> {
        let mut next = i.next;
        match (&i.opcode,&i.operand) {
   (0x800,Operand::Integer(n))=>self.push(Value::Integer(*n))?,
   (0x801,Operand::String(b))=>{let mut v=Vec::with_capacity(b.len()+1);v.extend_from_slice(b);v.push(0);self.push(Value::String(v))?},
   (0x803,Operand::Float(n))=>self.push(Value::Float(*n))?,
   (0x83a,Operand::Line(n))=>self.line=*n,
   (0x829,Operand::Types(types))=>{ensure!(self.frames.last().unwrap().locals.len()+types.len()<=MAX_LOCALS,"Majiro local budget");let v=types.iter().map(|t|Value::default_type(*t)).collect::<Result<Vec<_>>>()?;self.frames.last_mut().unwrap().locals.extend(v);},
   //40D390 checks the required leading arguments and retains variadic extras.
   (0x836,Operand::Types(types))=>{let a=&self.frames.last().unwrap().arguments;ensure!(types.len()<=a.len() && types.iter().zip(a.iter().rev()).all(|(t,v)|*t==v.kind()),"Majiro function argument signature");},
   (0x802,Operand::Variable{flags,hash,offset})=>{
    let mut value=self.variable(*flags,*hash,*offset)?;let modify=flags&7;let invert=(flags>>3)&3;
    ensure!(self.stack.len()<MAX_STACK,"Majiro stack budget");
    let changed=if modify!=0{let n=value.integer()?;let updated=match modify{1|3=>n.wrapping_add(1),2|4=>n.wrapping_sub(1),_=>bail!("Majiro variable modifier")};if modify<=2{value=Value::Integer(updated);}Some(Value::Integer(updated))}else{None};
    if invert!=0{value=match value{Value::Integer(n)=>Value::Integer(match invert{1=>n.wrapping_neg(),2=>i32::from(n==0),_=>!n}),Value::Float(n) if invert==1=>Value::Float(-n),_=>bail!("Majiro invert type")};}
    if let Some(changed)=changed{self.store(*flags,*hash,*offset,changed)?;}self.stack.push(value);
   },
   (0x837,Operand::Variable{flags,hash,offset})=>self.load_element(*flags,*hash,*offset)?,
   (op,Operand::Variable{flags,hash,offset}) if (0x270..=0x322).contains(op)=>self.store_element(*op,*flags,*hash,*offset)?,
   (0x843,Operand::Jump(target))=>{if let Some(jump)=self.bsel_set_click(*target){next=jump;}},
   (0x83b,Operand::Jump(target))=>{if let Some(jump)=self.bsel_set_one(0,arrays::CLICK_LEFT,*target){next=jump;}},
   (0x83d,Operand::Jump(target))=>{if let Some(jump)=self.bsel_set_one(1,arrays::CLICK_RIGHT,*target){next=jump;}},
   (0x83c,Operand::Jump(target))=>{if let Some(jump)=self.bsel_set_one(2,arrays::CLICK_MIDDLE,*target){next=jump;}},
   (0x844,Operand::None)=>self.frames.last_mut().unwrap().bsel.click=[0;3],
   (0x845,Operand::Jump(target))=>self.frames.last_mut().unwrap().bsel.aip4=*target,
   (0x846,Operand::None)=>{if let Some(jump)=self.bsel_aip4(){next=jump;}},
   (0x847,Operand::Jump(target))=>self.frames.last_mut().unwrap().bsel.destructor=*target,
   (op,Operand::Variable{flags,hash,offset}) if (0x1b0..=0x265).contains(op)=>{
    let base=if *op>=0x210{0x210}else{0x1b0};let group=(*op-base)/8;let ty=((*op-base)%8) as u8;
    let rhs=self.tail(1)?[0].clone();let rhs=if group==0 && ty==1 && rhs.kind()==0 {Value::Float(scalar::number(&rhs)? as f32)}else{rhs};ensure!(rhs.kind()==ty || (ty==1 && rhs.kind()==0),"Majiro assignment type");
    let value=if group==0{rhs}else{let old=self.variable(*flags,*hash,*offset)?;scalar::binary(match group{1=>0x100,2=>0x108,3=>0x110,4=>0x118,5=>0x120,6=>0x130,7=>0x128,8=>0x180,9=>0x168,10=>0x188,_=>bail!("Majiro assignment group")}|u16::from(ty),&old,&rhs)?};
    self.store(*flags,*hash,*offset,value.clone())?;self.stack.pop();if base==0x1b0{self.stack.push(value);}
   },
   (op,Operand::None) if *op>=0x100 && *op<=0x188=>{
    let a=self.tail(2)?;let value=scalar::binary(*op,&a[0],&a[1])?;self.stack.truncate(self.stack.len()-2);self.stack.push(value);
   },
   (0x190|0x198|0x1a0,Operand::None)=>{let n=self.tail(1)?[0].integer()?;let value=match i.opcode{0x190=>i32::from(n==0),0x198=>!n,_=>n.wrapping_neg()};*self.stack.last_mut().unwrap()=Value::Integer(value);},
   (0x1a1,Operand::None)=>{let Value::Float(n)=self.tail(1)?[0] else {bail!("Majiro neg.r requires float")};*self.stack.last_mut().unwrap()=Value::Float(-n);},
   (0x83e,Operand::None)=>{let value=scalar::convert_integer(&self.tail(1)?[0])?;*self.stack.last_mut().unwrap()=value;},
   (0x83f,Operand::None)=>{let value=self.tail(1)?[0].integer()?;*self.stack.last_mut().unwrap()=Value::Float(value as f32);},
   (0x191|0x1a8|0x1a9,Operand::None)=>{},
   (0x82f,Operand::None)=>{self.tail(1)?;self.stack.pop();},
   (0x82c,Operand::Jump(target))=>next=*target,
   (0x82d|0x82e,Operand::Jump(target))=>{if scalar::branch(&self.tail(1)?[0],i.opcode==0x82d){next=*target;}self.stack.pop();},
   (0x830,Operand::Jump(target))=>{
    //40D630 stores the selector in the actual thread-local native variable,
    //then pops it. Nested switch statements share this original cell.
    let value=self.tail(1)?[0].clone();
    self.store(2<<5,0x11f91fd3,0,value)?;
    self.stack.pop();next=*target;
   },
   (op @ (0x831..=0x833|0x838|0x839),Operand::Jump(target))=>{
    let value=&self.tail(1)?[0];
    let selector=self.banks[2].get(&0x11f91fd3).context("Majiro case selector absent")?;
    let take=match (selector,value) {
     (Value::Integer(a),Value::Integer(b))=>match *op {0x831=>a!=b,0x832=>a>=b,0x833=>a<=b,0x838=>a<b,_=>a>b},
     (Value::Float(a),Value::Float(b))=>match *op {0x831=>a!=b,0x832=>a>=b,0x833=>a<=b,0x838=>a<b,_=>a>b},
     _=>bail!("Majiro native case type not connected"),
    };
    //Actual40D670/40D710/40D790/40D810/40D890 only peek the
    //comparison value; contrary to the public opcode table they do not pop.
    if take {next=*target;}
   },
   (0x850,Operand::Switch(targets))=>{
    ensure!(!targets.is_empty(),"Majiro native switch has no default target");
    let index=self.tail(1)?[0].integer()?;
    let index=if index<0 || index as usize>=targets.len(){targets.len()-1}else{index as usize};
    next=targets[index];self.stack.pop();
   },
   (0x80f|0x810,Operand::Call{hash,arguments})=>{
    ensure!(self.frames.len()<MAX_FRAMES,"Majiro call depth budget");let (module,target)=self.resolve_call(*hash,i.pc)?;
    let args=self.tail(*arguments as usize)?.to_vec();let base=self.stack.len()-args.len();
    let serial=self.frame_serial()?;self.frames.push(Frame{serial,interrupted:None,global_initializer:false,return_pc:i.next,return_module:self.active_module,stack_base:base,arguments:args,locals:vec![],discard_return:i.opcode==0x810,bsel:Bsel::default()});self.stack.truncate(base);self.active_module=module;self.program=self.modules[module].program.clone();next=target;
   },
   (0x82b,Operand::None) if self.frames.last().unwrap().bsel.destructor!=0=>{
    //40D490: a registered destructor runs before the frame is popped.
    let f=self.frames.last_mut().unwrap();next=f.bsel.destructor;f.bsel.destructor=0;
   },
   (0x82b,Operand::None)=>{
    let value=if self.frames.last().unwrap().discard_return{None}else{Some(self.tail(1)?[0].clone())};
    //40D490 destroys every callee temporary above the caller's stack base,
    //then preserves only the top value for a function. Native case helpers
    //leave comparison values behind; they are valid return-time temporaries.
    let f=self.frames.pop().unwrap();self.stack.truncate(f.stack_base);
    if f.global_initializer {self.finish_global_initializer();}
    self.pending=f.interrupted;
    if self.frames.is_empty(){if let Some(value)=value{self.stack.push(value);}self.halted=true;}else{if let Some(value)=value{self.stack.push(value);}self.active_module=f.return_module;self.program=self.modules[f.return_module].program.clone();next=f.return_pc;}
   },
   (0x834|0x835,Operand::Call{hash,arguments})=>{let args=self.tail(*arguments as usize)?.iter().rev().cloned().collect();let event=Event::Native{pc:self.pc,hash:*hash,arguments:args,returns:i.opcode==0x834};return Ok(Some(self.queue(event,i.next,*arguments as usize)));},
   (0x840,Operand::String(bytes))=>return Ok(Some(self.queue(Event::Text{pc:self.pc,bytes:bytes.clone()},i.next,0))),
   (0x841,Operand::None)=>return Ok(Some(self.queue(Event::ProcessText{pc:self.pc},i.next,0))),
   (0x842,Operand::String(bytes))=>return Ok(Some(self.text_control(bytes,i.next)?)),
   _=>bail!("unsupported original Majiro instruction {:#x}; execution stopped without advancing",i.opcode),
  }
        self.pc = next;
        self.executed += 1;
        Ok(None)
    }
}
fn integer_binary(op: u16, a: &Value, b: &Value) -> Result<Value> {
    ensure!(
        op & 7 == 0,
        "Majiro noninteger arithmetic profile unsupported"
    );
    let a = a.integer()?;
    let b = b.integer()?;
    let result = match op {
        0x100 => a.wrapping_mul(b),
        //Actual40D950/40D9F0 have explicit zero-divisor branches. Keep
        //x86 wrapping multiplication; INT_MIN/-1 still raises the original
        //CPU divide exception and is not silently treated as wrapping division.
        0x108 => if b == 0 { a.wrapping_mul(10000) } else {
            a.checked_div(b).context("Majiro native signed division overflow")?
        },
        0x110 => if b == 0 { 0 } else {
            a.checked_rem(b).context("Majiro native signed remainder overflow")?
        },
        0x118 => a.wrapping_add(b),
        0x120 => a.wrapping_sub(b),
        0x128 => a.wrapping_shr(b as u32),
        0x130 => a.wrapping_shl(b as u32),
        0x138 => i32::from(a <= b),
        0x140 => i32::from(a < b),
        0x148 => i32::from(a >= b),
        0x150 => i32::from(a > b),
        0x158 => i32::from(a == b),
        0x160 => i32::from(a != b),
        0x168 => a ^ b,
        0x170 => i32::from(a != 0 && b != 0),
        0x178 => i32::from(a != 0 || b != 0),
        0x180 => a & b,
        0x188 => a | b,
        _ => bail!("Majiro integer operator"),
    };
    Ok(Value::Integer(result))
}
