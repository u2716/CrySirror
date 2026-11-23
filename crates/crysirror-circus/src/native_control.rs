//!447D20 copied execution fields mapped to shared externally clocked channels.
//! Heap queue banks remain owned by the current engine actor, not a save file.
use anyhow::{Context, Result, ensure};
use crysirror_scene::layer::queued::{Active, Lane, Record, SequenceStatus, SequenceTrack};
use std::collections::BTreeMap;
fn word(record: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(record[offset - 8..offset - 4].try_into().unwrap())
}
fn f(record: &[u8], offset: usize) -> Result<f32> {
    let value = f32::from_bits(word(record, offset));
    ensure!(
        value.is_finite(),
        "Circus nonfinite copied sprite control at{offset}"
    );
    Ok(value)
}
fn owned_float(record:&[u8],offset:usize,current:&SequenceStatus,auxiliary:bool)->Result<f32>{
    let value=f32::from_bits(word(record,offset));
    let channel=if auxiliary {match offset {
        36|424=>Some(0),40=>Some(1),44|416=>Some(2),48|420=>Some(3),_=>None,
    }}else{match offset {
        8|216=>Some(0),12|220=>Some(1),112|344=>Some(2),
        128|384=>Some(3),132=>Some(4),144|464=>Some(5),
        116|596|624=>Some(10),120=>Some(11),_=>None,
    }};
    let source_lane=channel.and_then(|channel|if auxiliary{Some(5)}else{match channel {
        0|1=>Some(0),2=>Some(3),3|4=>Some(4),5=>Some(6),10|11=>Some(10),_=>None,
    }});
    //447D20 explicitly inserts a memset-zero84B bank0 record for each
    //copied active lane. Its divisor20 is0, independently of the current
    //actor's previous heap. Raw execution velocity/values stay in this960B.
    let copied_zero=source_lane.is_some_and(|lane|word(record,632+lane*4)!=0);
    let reciprocal_ieee=!auxiliary&&matches!(offset,116|120|596|624)&&word(record,960)==4;
    ensure!(value.is_finite()||channel.is_some_and(|channel|current.allows_nonfinite_channel(channel))||copied_zero||reciprocal_ieee,
        "Circus copied nonfinite field{offset} lacks zero-record provenance");
    Ok(value)
}
fn inherit_zero_divisors(state:&mut SequenceStatus,current:&SequenceStatus,raw:&[u8],auxiliary:bool){
    for (id,lane) in &mut state.lanes {
        let original=current.lanes.get(id).and_then(|lane|lane.control.as_ref().or(lane.last_record.as_ref()));
        let original_zero=original.is_some_and(|record|matches!(record.track,SequenceTrack::Linear{velocity_divisor:Some(0.),..}));
        let original_ieee=original.is_some_and(|record|matches!(record.track,SequenceTrack::Linear{ieee_inputs:true,..}));
        let source_id=if auxiliary{5}else{usize::from(*id)};
        let copied_zero=word(raw,632+source_id*4)!=0;
        if !original_zero&&!copied_zero&&!original_ieee {continue}
        for record in [&mut lane.control,&mut lane.last_record].into_iter().flatten(){
            if let SequenceTrack::Linear{velocity_divisor,ieee_inputs,..}=&mut record.track {
                if original_ieee {*ieee_inputs=true;}
                else if original_zero||copied_zero {*velocity_divisor=Some(0.);}
            }
        }
    }
}
fn linear(
    channels: &[u8],
    target: Vec<f32>,
    velocity: Vec<f32>,
    multiplier: f32,
    first: bool,
    zero: bool,
    fast: bool,
) -> Result<(Record, Active)> {
    let origin = vec![0.; channels.len()];
    let ieee_inputs=target.iter().chain(velocity.iter()).any(|value|!value.is_finite());
    let track=SequenceTrack::Linear {
        channels:channels.to_vec(),target,ticks:1,velocity_divisor:None,ieee_inputs,
        references:channels.to_vec(),loop_origins:channels.to_vec(),
        delta:crysirror_scene::layer::queued::DeltaSource::Current,
        looping:false,start:None,complete_first:first,complete_zero:zero,
        fast,fast_channels:None,skip_zero:false,capture_zero:true,
    };
    Ok((
        Record {
            progress: None,
            track,
            velocity: velocity.clone(),
            origin,
        },
        Active::Linear {
            velocity,
            multiplier,
        },
    ))
}
fn lane(record: &[u8], source: u8, control: Record, active: Active) -> Result<Lane> {
    let flag = word(record, 632 + usize::from(source) * 4);
    ensure!(flag <= 1, "Circus copied sprite active flag");
    let cursor = word(record, 676 + usize::from(source) * 4);
    ensure!(cursor <= 100, "Circus copied sprite bank cursor");
    Ok(Lane {
        cursor: cursor as u16,
        records: BTreeMap::new(),
        active: (flag != 0).then_some(active.clone()),
        control: Some(control.clone()),
        last_record: Some(control),
        last_active: Some(active),
    })
}
/// Numerical primary controls only. PCK9 is reconstructed through its actual
/// resource/447AC0 restart, and remains the prepared current actor's lane.
pub(crate) fn from_record(record: &[u8], current: &SequenceStatus) -> Result<SequenceStatus> {
    ensure!(
        record.len() == 960 && current.values.len() == 17 && current.retained.len() == 17,
        "Circus copied primary control extent"
    );
    let mut state = current.clone();
    let f=|record:&[u8],offset:usize|owned_float(record,offset,current,false);
    for (channel, offset) in [
        (0, 8),
        (1, 12),
        (2, 112),
        (3, 128),
        (4, 132),
        (5, 144),
        (10, 116),
        (11, 120),
        (12, 152),
        (13, 156),
        (16, 32),
    ] {
        state.values[channel] = f(record, offset)?;
    }
    state.values[14] = f(record, 104)? - f(record, 96)?;
    state.values[15] = f(record, 108)? - f(record, 100)?;
    state.retained = state.values.clone();
    for (channel, offset) in [(0, 56), (1, 60), (2, 124), (3, 136), (4, 136), (5, 148)] {
        state.retained[channel] = f(record, offset)?;
    }
    let (control, active) = linear(
        &[0, 1],
        vec![f(record, 208)?, f(record, 212)?],
        vec![f(record, 216)?, f(record, 220)?],
        f(record, 228)?,
        false,
        true,
        true,
    )?;
    state.lanes.insert(0, lane(record, 0, control, active)?);
    for (id, channel, phase, end, baseline, velocity, amplitude, multiplier, cosine) in [
        (1, 0, 232, 236, 248, 256, 260, 268, false),
        (2, 1, 272, 276, 292, 300, 296, 308, true),
    ] {
        let track: SequenceTrack = serde_json::from_value(
            serde_json::json!({"kind":"oscillation","channel":channel,"amplitude":f(record,amplitude)?,"start_degrees":f(record,phase)?,"end_degrees":f(record,end)?,"ticks":1,"cosine":cosine,"looping":false,"radians_per_degree":0.01745327777777778_f64,"fast":true}),
        )?;
        let delta = f(record, velocity)?;
        let control = Record {
            progress: None,
            track,
            velocity: vec![delta],
            origin: Vec::new(),
        };
        let active = Active::Oscillation {
            phase: f(record, phase)?,
            velocity: delta,
            baseline: f(record, baseline)?,
            multiplier: f(record, multiplier)?,
        };
        state.lanes.insert(id, lane(record, id, control, active)?);
    }
    for (id, channels, target, velocity, multiplier) in [
        (3, vec![2], 316, 344, 348),
        (4, vec![3, 4], 356, 384, 388),
        (6, vec![5], 436, 464, 468),
    ] {
        let (control, active) = linear(
            &channels,
            vec![f(record, target)?; channels.len()],
            vec![f(record, velocity)?; channels.len()],
            f(record, multiplier)?,
            true,
            false,
            true,
        )?;
        state.lanes.insert(id, lane(record, id, control, active)?);
    }
    let target = f(record, 596)?;
    let (control, active) = linear(
        &[10, 11],
        vec![target; 2],
        vec![f(record, 624)?; 2],
        f(record, 628)?,
        true,
        false,
        true,
    )?;
    let mut saved = lane(record, 10, control, active)?;
    // The active reciprocal lane's copied start is already in116/120.
    if let Some(Record {
        track: SequenceTrack::Linear { start, .. },
        ..
    }) = &mut saved.control
    {
        *start = Some(vec![f(record, 116)?, f(record, 120)?]);
    }
    saved.last_record = saved.control.clone();
    state.lanes.insert(10, saved);
    for id in [7u8, 8] {
        let flag = word(record, 632 + usize::from(id) * 4);
        let cursor = word(record, 676 + usize::from(id) * 4);
        ensure!(
            flag <= 1 && cursor <= 100,
            "Circus copied heap-control flag/cursor"
        );
        let existing = state.lanes.entry(id).or_default();
        existing.cursor = cursor as u16;
        if flag != 0 {
            let control = existing
                .records
                .get(&(cursor as u8))
                .cloned()
                .unwrap_or_else(|| Record {
                    progress: None,
                    track: SequenceTrack::Frames {
                        channel: id,
                        frames: if id == 7 { 1 } else { 0 },
                        hold_ticks: -1,
                        looping: false,
                        capture: id == 7,
                    },
                    velocity: Vec::new(),
                    origin: Vec::new(),
                });
            let active = control.progress.clone().unwrap_or(Active::Frames {
                frame: 0,
                counter: 0,
                attached: true,
            });
            ensure!(
                matches!(control.track, SequenceTrack::Frames { .. }),
                "Circus retained heap queue has a different role"
            );
            existing.active = Some(active.clone());
            existing.control = Some(control.clone());
            existing.last_record = Some(control);
            existing.last_active = Some(active);
        } else {
            existing.active = None;
            existing.control = None;
        }
    }
    inherit_zero_divisors(&mut state,current,record,false);
    state.complete = state
        .lanes
        .values()
        .all(|lane| lane.active.is_none() && lane.records.is_empty());
    Ok(state)
}
/// The same renderer's auxiliary view lane is owned by its separate shared
/// view actor. Its four channels have source flags652/cursor696.
pub(crate) fn auxiliary_from_record(
    record: &[u8],
    current: &SequenceStatus,
) -> Result<SequenceStatus> {
    ensure!(
        record.len() == 960 && current.values.len() == 4,
        "Circus auxiliary copied-control extent"
    );
    let mut state = current.clone();
    let f=|record:&[u8],offset:usize|owned_float(record,offset,current,true);
    state.values = [36, 40, 44, 48]
        .map(|at| f(record, at))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let target = [
        f(record, 396)?,
        f(record, 396)?,
        f(record, 408)?,
        f(record, 412)?,
    ];
    state.retained = target.to_vec();
    let (control, active) = linear(
        &[0, 1, 2, 3],
        target.to_vec(),
        vec![
            f(record, 424)?,
            f(record, 424)?,
            f(record, 416)?,
            f(record, 420)?,
        ],
        f(record, 428)?,
        false,
        false,
        false,
    )?;
    let mut control = control;
    if let SequenceTrack::Linear {
        fast,
        fast_channels,
        ..
    } = &mut control.track
    {
        *fast = true;
        *fast_channels = Some(vec![2, 3]);
    }
    state.lanes.insert(0, lane(record, 5, control, active)?);
    inherit_zero_divisors(&mut state,current,record,true);
    state.complete = state
        .lanes
        .values()
        .all(|lane| lane.active.is_none() && lane.records.is_empty());
    Ok(state)
}

///4432E0's alignment mode is a projection of independent position/scale
///controls, not a rewrite of their values or a resized texture rectangle.
pub(crate) fn alignment_maps(record:&[u8])->Result<[crysirror_scene::layer::queued::ValueMap;2]> {
    use crysirror_scene::layer::queued::ValueMap;
    ensure!(record.len()==960,"Circus alignment record extent");
    let mode=word(record,964);
    if mode==0{return Ok([ValueMap::Identity,ValueMap::Identity])}
    let extent=[f(record,104)?-f(record,96)?,f(record,108)?-f(record,100)?];
    Ok(std::array::from_fn(|axis|ValueMap::CenteredScale{source_extent:extent[axis],reference_extent:if mode==2{extent[axis]}else{[1280.,720.][axis]},scale_channel:[3,4][axis],test_channels:vec![3,4],threshold:1000.,below:100.,above:10000.}))
}
