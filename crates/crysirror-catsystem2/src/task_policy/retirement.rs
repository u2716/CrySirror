// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed3/67/endfes native lifetime policy: mark, then dispose at the next
//! original child phase. Engine resources are retired in a real atomic batch.
use super::*;
use crysirror_core::{
    service::{EngineQuery, HostServices},
    state::ClientId,
};
impl NativeTaskPolicy {
    pub(crate) fn request_fes_destroy(
        &mut self,
        id: u32,
        interpreter: Option<&mut crate::native_interpreter::Interpreter>,
    ) -> Result<()> {
        let task = self
            .fes_tasks
            .get_mut(&id)
            .context("CatSystem2 destroy target is not a supported FES task")?;
        ensure!(
            task.auxiliary_records.is_empty(),
            "CatSystem2 FES object-destroy callbacks require actual shared object bindings"
        );
        if task.player_loaded {
            let interpreter = interpreter.context("CatSystem2 destroy interpreter missing")?;
            ensure!(
                interpreter.select_player_bank(id),
                "CatSystem2 FES destroy local bank missing"
            );
            //62BED0/3 writes the child's own named value; task20 remains intact.
            interpreter.set_named_value(b"_EXITCODE_", 0, u32::MAX)?;
        }
        task.deletion_pending = true;
        Ok(())
    }
    pub(crate) fn end_fes_child(
        &mut self,
        owner: u32,
        index: usize,
        interpreter: &mut crate::native_interpreter::Interpreter,
    ) -> Result<()> {
        ensure!(index < 2, "CatSystem2 native child slot index");
        // 629850 case 5 sends the KCS task its destroy message; the adapter owns its VM.
        if index == 1 {
            if let Some(child) = self.fes(owner)?.child_tasks[1] {
                self.kcs_retired.push(child);
                self.fes_tasks.get_mut(&owner).unwrap().child_tasks[1] = None;
            }
        }
        if let Some(child) = self.fes(owner)?.child_tasks[index] {
            ensure!(
                self.fes(child)?.parent == owner,
                "CatSystem2 native child ancestry mismatch"
            );
            self.request_fes_destroy(child, Some(&mut *interpreter))?;
            self.fes_tasks.get_mut(&owner).unwrap().child_tasks[index] = None;
        }
        ensure!(
            interpreter.select_player_bank(owner),
            "CatSystem2 end-child parent local bank missing"
        );
        //77A7C0/77AAA0 reset the PARENT before629850 processes queued3/5.
        interpreter.set_named_value(b"_EXITCODE_", 0, u32::MAX)
    }
    pub(crate) fn retire_fes_subtree(
        &mut self,
        id: u32,
        host: &mut dyn HostServices,
        client: ClientId,
        mut interpreter: Option<&mut crate::native_interpreter::Interpreter>,
        protected: &[u32],
    ) -> Result<()> {
        ensure!(
            self.fes(id)?.deletion_pending,
            "CatSystem2 FES retirement was not requested"
        );
        fn visit(
            policy: &NativeTaskPolicy,
            id: u32,
            depth: usize,
            out: &mut Vec<u32>,
        ) -> Result<()> {
            ensure!(depth < 64, "CatSystem2 FES retirement ancestry depth");
            for child in policy.fes_tasks.values().filter(|t| t.parent == id) {
                visit(policy, child.id, depth + 1, out)?;
            }
            out.push(id);
            Ok(())
        }
        let mut ids = Vec::new();
        visit(self, id, 0, &mut ids)?;
        let selected = ids.iter().copied().collect::<BTreeSet<_>>();
        ensure!(
            !protected.iter().any(|id| selected.contains(id)),
            "CatSystem2 cannot retire an owner of a retained resource continuation"
        );
        ensure!(!self.planes.values().any(|t|selected.contains(&t.parent))
            &&!self.image_lists.values().any(|t|selected.contains(&t.parent)),
            "CatSystem2 owned Plane/image-list destruction requires their reviewed resource retirement");
        ensure!(
            !self
                .fes_tasks
                .values()
                .filter(|t| !selected.contains(&t.id))
                .any(|t| t.child_tasks.iter().flatten().any(|c| selected.contains(c))),
            "CatSystem2 external deletion leaves an original live child reference"
        );
        for id in &ids {
            let task = self.fes(*id)?;
            ensure!(
                task.auxiliary_records.is_empty() && task.preload_names.is_empty(),
                "CatSystem2 FES object/preload resource retirement remains unsupported"
            );
            ensure!(
                task.child_tasks[1].is_none(),
                "CatSystem2 KCS-child resource retirement remains unsupported"
            );
        }
        let buttons = self
            .buttons
            .values()
            .filter(|b| selected.contains(&b.parent))
            .map(|b| b.id)
            .collect::<Vec<_>>();
        let mut commands = crysirror_scene::ServiceBatch::default();
        for button in &buttons {
            if self.button(*button)?.binding_target.is_some() {
                commands
                    .input_routes
                    .push(crysirror_scene::input_routes::InputRouteCommand::Remove { id: *button });
            }
        }
        let roots = ids
            .iter()
            .filter_map(|id| self.fes_tasks[id].object_root)
            .collect::<BTreeSet<_>>();
        //Remove only topmost actual engine roots; a developer may have used
        //the public scene API to parent two native logical groups together.
        for root in &roots {
            let mut current = *root;
            let mut nested = false;
            let mut depth = 0;
            loop {
                let layer: Option<crysirror_scene::layer::LayerSpec> = serde_json::from_slice(
                    &host.query_engine(client, EngineQuery::Layer(current))?,
                )?;
                let layer = layer.context("CatSystem2 FES retirement shared layer missing")?;
                let Some(parent) = layer.parent else {
                    break;
                };
                if roots.contains(&parent) {
                    nested = true;
                    break;
                }
                depth += 1;
                ensure!(depth < 64, "CatSystem2 retirement engine ancestry depth");
                current = parent;
            }
            if !nested {
                commands
                    .layers
                    .push(crysirror_scene::layer::LayerCommand::Remove { id: *root });
            }
        }
        let mut proposed = self.clone();
        let mut changed_lists=BTreeSet::new();
        for id in &ids {
            let task=self.fes(*id)?;
            let Some(bank)=&task.object_bank else{continue;};
            if let Some(list_id)=task.image_list {
                let list=proposed.image_lists.get_mut(&list_id).context("CatSystem2 retiring class ImageList missing")?;
                if let Some(publication)=&mut list.publication {
                    publication.clear_sender(*id)?;
                    let parents=bank.objects.keys().map(|object|0x4d000000u32.checked_add(*object).context("CatSystem2 retiring class Layer range")).collect::<Result<BTreeSet<_>>>()?;
                    for plane in publication.take_planes(&parents){
                        commands.canvases.push(crysirror_scene::canvas::CanvasCommand::Hide{layer:plane.presentation});
                        commands.canvases.push(crysirror_scene::canvas::CanvasCommand::Destroy{id:plane.canvas});
                    }
                    changed_lists.insert(list_id);
                }
            }
            // A class Plane may publish through another list (Main's): its layer goes with the root.
            let parents=bank.objects.keys().map(|object|0x4d000000u32.checked_add(*object).context("CatSystem2 retiring class Layer range")).collect::<Result<BTreeSet<_>>>()?;
            for (list_id,list) in proposed.image_lists.iter_mut(){
                let Some(publication)=&mut list.publication else{continue;};
                let planes=publication.take_planes(&parents);
                if planes.is_empty(){continue;}
                for plane in planes{
                    commands.canvases.push(crysirror_scene::canvas::CanvasCommand::Hide{layer:plane.presentation});
                    commands.canvases.push(crysirror_scene::canvas::CanvasCommand::Destroy{id:plane.canvas});
                }
                changed_lists.insert(*list_id);
            }
            proposed.image_retirements.extend(task.class_image_leases.iter().cloned());
        }
        for list in changed_lists {
            commands.canvases.extend(proposed.image_lists[&list].publication.as_ref().unwrap().commands()?);
        }
        let mut vars = interpreter.as_deref().cloned();
        for id in &ids {
            let task = proposed.fes(*id)?;
            let image_list = task.image_list;
            let loaded = task.player_loaded;
            if let Some(list) = image_list {
                proposed.set_image_mode(list, 1)?;
            }
            if loaded {
                ensure!(
                    vars.as_mut()
                        .context("CatSystem2 retirement interpreter missing")?
                        .delete_player_bank(*id),
                    "CatSystem2 FES retirement local bank missing"
                );
            }
        }
        for id in buttons.iter().chain(ids.iter()) {
            proposed.buttons.remove(id);
            proposed.fes_tasks.remove(id);
            ensure!(
                proposed.slots.remove(id),
                "CatSystem2 native retirement task slot missing"
            );
            proposed.results.destroy(*id).map_err(anyhow::Error::msg)?;
        }
        if !commands.layers.is_empty() || !commands.input_routes.is_empty() || !commands.canvases.is_empty() {
            host.submit_engine(client, 1, &serde_json::to_vec(&commands)?)?;
        }
        *self = proposed;
        if let Some(vars) = vars {
            *interpreter.as_deref_mut().unwrap() = vars;
        }
        self.flush_retired_images(host,client)?;
        Ok(())
    }
    pub(crate) fn flush_retired_images(&mut self,host:&mut dyn HostServices,client:ClientId)->Result<()> {
        while let Some(path)=self.image_retirements.last(){
            host.release_image_resource(client,path)?;
            self.image_retirements.pop();
        }Ok(())
    }
}
