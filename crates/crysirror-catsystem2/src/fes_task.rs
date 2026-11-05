// SPDX-License-Identifier: MPL-2.0
//! Original627E50 task state. This is native resource/scheduling policy, not a
//! hosted FES renderer. The player and its operations remain separately gated.
use anyhow::{ensure, Result};

#[derive(Clone, Debug)]
pub(crate) struct FesTask {
    pub id: u32,
    pub parent: u32,
    pub name: Vec<u8>,
    pub parameters: Vec<u8>,
    /// Parent177 returns the original raw-string bank, not the interpreter.
    pub string_bank_owner: Option<u32>,
    pub interpreter_owner: Option<u32>,
    pub player_loaded: bool,
    pub player: Option<crate::fes_program::Machine>,
    ///Original constructor/OBJECT data; no pixels, engine handles or device
    ///pointers. Its resource phase belongs to the retained owner frame.
    pub object_bank: Option<crate::fes_object_bank::Bank>,
    ///Scoped published image paths only. Resource release is delegated to the
    ///shared host, which preserves older Canvas/snapshot readers.
    pub class_image_leases:Vec<String>,
    /// Shared transparent Layer group for754310's implicit BASE _TOPOBJ_.
    /// It owns reusable transform/motion state, not a private composed screen.
    pub object_root: Option<u32>,
    pub running: bool,
    /// Button-pass values for `_CLICK_L_`/`_CLICK_R_`, written before the next step.
    pub click_values: Vec<(&'static [u8], u32)>,
    pub deletion_pending:bool,
    pub paused: bool,
    pub first_tick: bool,
    pub low_button: Option<u32>,
    /// Previous frame left and right button state for BUTTON edges.
    pub button_down: [bool; 2],
    pub direction_revision: Option<u64>,
    /// Button events not yet given to the player; one section runs at a time.
    pub button_queue: std::collections::VecDeque<crate::fes_button::Dispatch>,
    pub image_list: Option<u32>,
    pub native_rect: [i32; 4],
    pub upload_position: Option<[i32; 2]>,
    /// Constructor125 cannot reach INPUT before attach. Output remains unknown;
    /// do not invent a pointer location or INPUT task reference.
    pub saved_cursor: Option<[i32; 2]>,
    pub input_task: Option<u32>,
    pub sound_task: Option<u32>,
    pub child_tasks: [Option<u32>; 2],
    pub exit_code: i32,
    pub selected_object: i32,
    pub click_state: u32,
    pub message_parameter: u32,
    pub script_name: Vec<u8>,
    pub next_object: [u8; 36],
    pub previous_object: [u8; 36],
    pub resource_directory: Vec<u16>,
    pub auxiliary_records: Vec<u32>,
    pub preload_names: Vec<Vec<u8>>,
}
impl FesTask {
    pub fn new(id: u32, parent: u32, name: Vec<u8>, parameters: Vec<u8>) -> Result<Self> {
        ensure!(
            parameters.len() < 32 && !parameters.contains(&0),
            "CatSystem2 native FES parameter extent"
        );
        Ok(Self {
            id,
            parent,
            name,
            parameters,
            string_bank_owner: Some(parent),
            interpreter_owner: None,
            player_loaded: false,
            player: None,
            object_bank: None,
            class_image_leases:Vec::new(),
            object_root: None,
            running: false,
            click_values: Vec::new(),
            deletion_pending:false,
            paused: false,
            first_tick: true,
            low_button: None,
            button_down: [false; 2],
            button_queue: Default::default(),
            image_list: None,
            native_rect: [0; 4],
            //627E50 explicitly zeroes1108/1112, unlike image-list112/116.
            upload_position: Some([0, 0]),
            saved_cursor: None,
            direction_revision: None,
            input_task: None,
            sound_task: None,
            child_tasks: [None, None],
            exit_code: -1,
            selected_object: -1,
            click_state: 0,
            //Native66 sends26 after attach;62AD10 stores this[298]=lparam0.
            message_parameter: 0,
            script_name: Vec::new(),
            next_object: [0; 36],
            previous_object: [0; 36],
            //Root5DD3C0's UTF16 constant8CD268 is exactly".". The scoped
            //asset root supplies its host location; no native pointer is kept.
            resource_directory: vec![u16::from(b'.')],
            auxiliary_records: Vec::new(),
            preload_names: Vec::new(),
        })
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        let mut value=serde_json::json!({"id":self.id,"task_parent":self.parent,"name":String::from_utf8_lossy(&self.name),"parameters":self.parameters,"string_bank_owner":self.string_bank_owner,"interpreter_owner":self.interpreter_owner,"player_loaded":self.player_loaded,"object_root":self.object_root,"player":self.player.as_ref().map(crate::fes_program::Machine::diagnostics),"running":self.running,"paused":self.paused,"first_tick":self.first_tick,"low_button":self.low_button,"image_list":self.image_list,"native_rect":self.native_rect,"upload_position":self.upload_position,"saved_cursor":self.saved_cursor,"input_task":self.input_task,"sound_task":self.sound_task,"child_tasks":self.child_tasks,"exit_code":self.exit_code,"selected_object":self.selected_object,"click_state":self.click_state,"message_parameter":self.message_parameter,"script_name":self.script_name,"next_object":self.next_object.as_slice(),"previous_object":self.previous_object.as_slice(),"resource_directory_utf16":self.resource_directory,"auxiliary_records":self.auxiliary_records.len(),"preload_names":self.preload_names.len()});
        value["deletion_pending"]=serde_json::Value::Bool(self.deletion_pending);
        let detail=std::env::var_os("CRYSIRROR_CATSYSTEM2_OBJECTS").is_some();
        value["class_objects"]=serde_json::json!(self.object_bank.as_ref().map(|bank|bank.objects.values().map(|object|{
            let mut item=serde_json::json!({"id":object.id,"type":object.declared_type});
            if detail {
                item["name"]=serde_json::json!(String::from_utf8_lossy(&object.name));
                item["parent"]=serde_json::json!(object.parent);
                item["enabled"]=serde_json::json!(object.enabled);
                use crate::native_class_state::State;
                item["state"]=match &object.state {
                    State::Image(image)|State::CImage(image)=>serde_json::json!({"file":String::from_utf8_lossy(&image.filename),"frame":image.native_frame(),"position":image.pose.position,"scale":image.pose.scale,"visible":image.pose.visible,"alpha":image.pose.alpha,"size":image.size,"draw_id":image.draw_id,"mask":image.mask,"missing":image.missing}),
                    State::Plane(plane)|State::CPlane(plane)=>serde_json::json!({"position":plane.pose.position,"visible":plane.pose.visible,"size":plane.size,"vram":plane.vram,"text_prefix":plane.text.as_ref().map(|t|&t.text[..t.text.len().min(16)]),"glyphs":plane.text.as_ref().map(|t|t.glyphs.len())}),
                    _=>serde_json::Value::Null,
                };
            }
            item
        }).collect::<Vec<_>>()));
        value["class_image_leases"]=serde_json::json!(self.class_image_leases);
        value
    }
}
