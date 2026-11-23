// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native754310/7589E0/758F70 logical typed-object bank. No shared resource
//! receipt is implied by preparing this VM state; player admission stays gated.
use crate::{
    fes_declarations::{Declarations, Reference},
    fes_references::{self, IndexedName},
    motion_set::MotionSet,
    native_class_state::State,
    native_interpreter::Interpreter,
    native_values::StringVariables,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub(crate) struct Object {
    pub id: u32,
    pub name: Vec<u8>,
    pub index: u32,
    pub declared_type: u8,
    pub parent: Option<u32>,
    pub enabled: bool,
    pub state: State,
    pub motions: MotionSet,
}
#[derive(Clone, Debug)]
pub(crate) struct Bank {
    pub owner: u32,
    pub root: u32,
    pub key_blocks: Option<crate::fes_key_blocks::KeyBlocks>,
    counts: BTreeMap<Vec<u8>, u32>,
    names: BTreeMap<(Vec<u8>, u32), u32>,
    pub objects: BTreeMap<u32, Object>,
    resource_order: Vec<u32>,
}
fn raw(reference: &Reference) -> Vec<u8> {
    let mut value = reference.name.clone();
    if let Some(index) = &reference.index {
        value.push(b'[');
        value.extend(index);
        value.push(b']');
    }
    value
}
impl Bank {
    pub(crate) fn motion_order(&self)->Result<Vec<u32>> {
        fn visit(bank:&Bank,id:u32,depth:usize,out:&mut Vec<u32>)->Result<()> {
            ensure!(depth<64,"CatSystem2 original motion hierarchy cycle/depth");
            for object in bank.objects.values().filter(|object|object.parent==Some(id)){
                visit(bank,object.id,depth+1,out)?;
            }
            out.push(id);Ok(())
        }
        let mut out=Vec::new();visit(self,self.root,0,&mut out)?;Ok(out)
    }
    pub(crate) fn active_subtree(&self,id:u32)->Result<bool> {
        fn active(bank:&Bank,id:u32,depth:usize)->Result<bool>{
            ensure!(depth<64,"CatSystem2 original motion active hierarchy cycle/depth");
            let object=bank.objects.get(&id).context("CatSystem2 original motion object missing")?;
            if object.motions.any_active()?{return Ok(true);}
            for child in bank.objects.values().filter(|object|object.parent==Some(id)){
                if active(bank,child.id,depth+1)?{return Ok(true);}
            }Ok(false)
        }active(self,id,0)
    }
    pub(crate) fn skip_motions(&mut self)->Result<()> {
        for id in self.motion_order()?{self.objects.get_mut(&id).unwrap().motions.skip_all()?;}
        Ok(())
    }
    ///7589E0 pushes DEFINE/index records through75C4D0;755D80 consumes
    ///that FIFO during6287B0 resource initialization. The name tree exists
    ///for lookup and must not reorder initialization or same-key later writes.
    pub fn ordered_ids(&self) -> Vec<u32> {
        std::iter::once(self.root)
            .chain(self.resource_order.iter().copied())
            .collect()
    }
    pub fn count(&self, name: &[u8]) -> Option<u32> {
        self.counts.get(name).copied()
    }
    pub fn lookup(&self, reference: &IndexedName) -> Option<u32> {
        let index = if reference.index == -1 {
            0
        } else {
            u32::try_from(reference.index).ok()?
        };
        self.names.get(&(reference.name.clone(), index)).copied()
    }
    ///7690E0 resolves one/all declared members before command dispatch. Unknown
    ///names/indices fail; they must not become default-zero flag expressions.
    pub fn targets(&self,raw:&[u8],vm:&mut Interpreter,strings:&StringVariables)->Result<Vec<u32>>{
        let reference=fes_references::evaluate(raw,vm,strings,&|name|self.count(name))?;
        let count=self.count(&reference.name).context("CatSystem2 object command name missing")?;
        let range=if reference.index==-1{0..count}else{
            // 754F90 misses and 7690E0 falls through to 7692C0's non-object evaluation: no member changes.
            let Ok(index)=u32::try_from(reference.index) else{return Ok(Vec::new());};
            if !self.names.contains_key(&(reference.name.clone(),index)){return Ok(Vec::new());}
            index..index+1
        };
        range.map(|index|self.names.get(&(reference.name.clone(),index)).copied().context("CatSystem2 object command missing member")).collect()
    }
    ///The direct original disp/size/pri families do not load code or invoke a
    ///generic interpolation shortcut. Commands with built-in motion scripts
    ///(including pos/color/stop) are deliberately handled by their own path.
    pub fn direct(&mut self,target:&[u8],arguments:&[Vec<u8>],vm:&mut Interpreter,strings:&StringVariables)->Result<()> {
        let mut staged=vm.clone();
        let ids=self.targets(target,&mut staged,strings)?;
        let command=arguments.first().context("CatSystem2 object command missing method")?;
        let words=&arguments[1..];
        let field=match command.as_slice(){
            b"disp"=>{ensure!(words.len()<=1,"CatSystem2 disp arguments");43},
            b"size"=>{ensure!(words.len()<=2,"CatSystem2 size arguments");39},
            b"pri"=>{ensure!(words.len()==1,"CatSystem2 pri arguments");41},
            _=>anyhow::bail!("CatSystem2 object method {} requires its original native/motion resource policy",String::from_utf8_lossy(command)),
        };
        let values=if command==b"size" {
            let mut out=Vec::new();for i in 0..2{
                if i>0{out.push(b',');}
                let value=if let Some(word)=words.get(i){staged.evaluate(word)?}else{0};
                out.extend(value.to_string().as_bytes());
            }out
        }else{
            let value=if let Some(word)=words.first(){staged.evaluate(word)?}else{1};
            (if command==b"disp"{u32::from(value!=0)}else{value}).to_string().into_bytes()
        };
        let fields=vec![(field,values)];
        let mut reference=|raw:&[u8],vm:&mut Interpreter|fes_references::evaluate(raw,vm,strings,&|name|self.count(name));
        let mut parent=|reference:&IndexedName|Ok(self.lookup(reference));
        let values=crate::fes_object_values::project(&fields,0,&mut staged,&mut reference,&mut parent)?;
        //Class-specific unsupported combinations fail before any member changes.
        let mut updated=Vec::new();
        for id in ids {
            let object=&self.objects[&id];
            // SAVEPANEL and other data classes accept disp with nothing to show.
            if matches!(object.state,State::Base){continue;}
            ensure!(matches!(object.state,State::CImage(_)|State::Image(_)|State::CPlane(_)|State::Plane(_)),"CatSystem2 direct visual method on nonvisual original class");
            let mut state=object.state.clone();state.apply(&values,object.index)?;updated.push((id,state));
        }
        for (id,state) in updated{self.objects.get_mut(&id).unwrap().state=state;}
        *vm=staged;Ok(())
    }
    ///Prepare on a cloned original Interpreter. Definitions are all constructed
    ///before OBJECT rows; one all-array row evaluates values once and then applies
    ///that result to every member. Failed native lookup/class policy never commits
    ///partially evaluated expressions or bogus named object-pointer values.
    pub fn prepare(
        owner: u32,
        declarations: &Declarations,
        interpreter: &mut Interpreter,
        strings: &StringVariables,
    ) -> Result<Self> {
        ensure!(owner < 1 << 20, "CatSystem2 typed bank owner handle range");
        let mut vm = interpreter.clone();
        ensure!(
            vm.select_player_bank(owner),
            "CatSystem2 typed object owner local bank unavailable"
        );
        let root = owner
            .checked_mul(4096)
            .and_then(|n| n.checked_add(1))
            .context("CatSystem2 object handle range")?;
        let top = b"_TOPOBJ_".to_vec();
        let mut bank = Self {
            owner,
            root,
            key_blocks: None,
            resource_order: Vec::new(),
            counts: BTreeMap::from([(top.clone(), 1)]),
            names: BTreeMap::from([((top.clone(), 0), root)]),
            objects: BTreeMap::from([(
                root,
                Object {
                    id: root,
                    name: top.clone(),
                    index: 0,
                    declared_type: 0,
                    parent: None,
                    enabled: true,
                    state: State::Base,
                    motions: Default::default(),
                },
            )]),
        };
        vm.set_named_value(&top, 0, root)?;
        //755D10 imports KEYBLOCK before DEFINE, even if the text places it later.
        for (_, block) in &declarations.key_blocks {
            ensure!(
                bank.key_blocks.is_none(),
                "CatSystem2 duplicate KEYBLOCK profile"
            );
            bank.key_blocks = Some(crate::fes_key_blocks::KeyBlocks::decode(block, &mut vm)?);
        }
        for (_, definition) in &declarations.definitions {
            let reference =
                fes_references::evaluate(&raw(&definition.reference), &mut vm, strings, &|name| {
                    bank.count(name)
                })?;
            ensure!(
                reference.index >= 0,
                "CatSystem2 native DEFINE all-array sentinel is not a count"
            );
            let count = (reference.index as u32).max(1);
            ensure!(
                count <= 4096
                    && bank
                        .objects
                        .len()
                        .checked_add(count as usize)
                        .is_some_and(|n| n <= 4096),
                "CatSystem2 typed object allocation budget"
            );
            //Unknown classes must fail before registering a name as a zero/default
            //expression. Constructors are original VM data, not original engine cores.
            let state = State::new(definition.kind)?;
            // 7544F0 makes every member and 755400 keeps the first per (name, index), so a
            // longer repeat adds members; the name's whole-array count stays the first one.
            bank.counts.entry(reference.name.clone()).or_insert(count);
            for index in 0..count {
                if bank.names.contains_key(&(reference.name.clone(), index)) {
                    continue;
                }
                let id = root
                    .checked_add(bank.objects.len() as u32)
                    .context("CatSystem2 typed object ID overflow")?;
                let object = Object {
                    id,
                    name: reference.name.clone(),
                    index,
                    declared_type: definition.kind,
                    parent: Some(root),
                    enabled: true,
                    state: state.clone(),
                    motions: Default::default(),
                };
                bank.names.insert((reference.name.clone(), index), id);
                bank.objects.insert(id, object);
                bank.resource_order.push(id);
                vm.set_named_value(&reference.name, index, id)?;
            }
        }
        for (_, assignment) in &declarations.assignments {
            let target =
                fes_references::evaluate(&raw(&assignment.target), &mut vm, strings, &|name| {
                    bank.count(name)
                })?;
            // 754F90 finds nothing for a row naming an undeclared object: the row is dropped.
            let Some(count) = bank.count(&target.name) else {
                continue;
            };
            let indices = if target.index == -1 {
                0..count
            } else {
                // 754F90 finds no such member: the row is dropped.
                let Ok(index) = u32::try_from(target.index) else { continue };
                if !bank.names.contains_key(&(target.name.clone(), index)) {
                    continue;
                }
                index..index + 1
            };
            let mut reference = |raw: &[u8], vm: &mut Interpreter| {
                fes_references::evaluate(raw, vm, strings, &|name| bank.count(name))
            };
            let mut parent = |reference: &IndexedName| Ok(bank.lookup(reference));
            let values = crate::fes_object_values::project(
                &assignment.fields,
                target.index,
                &mut vm,
                &mut reference,
                &mut parent,
            )
            .with_context(|| {
                format!(
                    "CatSystem2 OBJECT row target {} index {}",
                    String::from_utf8_lossy(&target.name),
                    target.index
                )
            })?;
            for index in indices {
                let id = bank.names[&(target.name.clone(), index)];
                let object = bank.objects.get_mut(&id).unwrap();
                //7590F0 applies parent before the class-specific callback. A genuinely
                //absent native parent detaches; it is not guessed to be _TOPOBJ_.
                if values.word(0) & 0x40 != 0 {
                    object.parent = values.parent;
                }
                object.state.apply(&values, index)?;
            }
        }
        *interpreter = vm;
        Ok(bank)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_declaration_order_and_all_array_rows_keep_original_expression_and_handles(
    ) -> Result<()> {
        let program=crate::fes_program::Program::parse(b"#DEFINE\nCIMAGE sprite[3]\nPLANE view\n#OBJECT POS PRI DISP\nsprite $0+=1,5 10 1\n#OBJECT PL\nsprite view\n#OBJECT PARENT\nsprite[1] sprite[0]\n#START\nwait 1",&||Ok(()))?;
        let mut vm = Interpreter::new(32)?;
        vm.create_player_bank(8)?;
        let bank = Bank::prepare(
            8,
            &program.declarations,
            &mut vm,
            &StringVariables::default(),
        )?;
        assert_eq!(vm.flag(0), 1, "one row must not execute per member");
        let root = bank.root;
        for index in 0..3 {
            let id = bank
                .lookup(&IndexedName {
                    name: b"sprite".to_vec(),
                    index,
                })
                .unwrap();
            let object = &bank.objects[&id];
            let State::CImage(state) = &object.state else {
                panic!()
            };
            assert_eq!(state.pose.position, [1., 5.]);
            assert_eq!(state.draw_id, 10 + index);
            assert_eq!(state.pose.priority, 30 + index);
            assert_eq!(state.plane.name, b"view");
            assert_eq!(state.plane.index, 0);
            assert_eq!(vm.evaluate(format!("sprite[{index}]").as_bytes())?, id);
            assert_eq!(
                object.parent,
                if index == 1 {
                    bank.lookup(&IndexedName {
                        name: b"sprite".to_vec(),
                        index: 0,
                    })
                } else {
                    Some(root)
                }
            );
        }
        assert_eq!(bank.count(b"SPRITE"), None);
        assert_eq!(bank.count(b"sprite"), Some(3));
        assert_ne!(root, 0);
        Ok(())
    }
    #[test]
    fn unsupported_class_lifetime_or_missing_target_keeps_original_interpreter_unchanged(
    ) -> Result<()> {
        let mut vm = Interpreter::new(32)?;
        vm.create_player_bank(8)?;
        vm.set_flag(0, 7);
        for source in [
            b"#DEFINE\nPARTICLE p[++$0]\n#START\nwait 1".as_slice(),
        ] {
            let program = crate::fes_program::Program::parse(source, &|| Ok(()))?;
            assert!(Bank::prepare(
                8,
                &program.declarations,
                &mut vm,
                &StringVariables::default()
            )
            .is_err());
            assert_eq!(vm.flag(0), 7);
            assert_eq!(vm.evaluate(b"p")?, 0);
        }
        // 754F90: a row naming an undeclared object is dropped; config.fes has one.
        let program = crate::fes_program::Program::parse(b"#DEFINE\nIMAGE p\n#OBJECT POS\nmissing 1,0\n#START\nwait 1", &|| Ok(()))?;
        assert!(Bank::prepare(8, &program.declarations, &mut vm, &StringVariables::default()).is_ok());
        Ok(())
    }
}
