//!4012D0's history-only narrow-string policy, reconstructed from the complete
//!4012D0..402009 instruction listing. This removes scheduled commands without
//!executing them; layout, ruby positioning and glyph rendering stay shared.
use anyhow::{Context,Result,ensure};
use encoding_rs::SHIFT_JIS;

fn encoded(text:&str)->Result<Vec<u8>> {
    let (bytes,_,error)=SHIFT_JIS.encode(text);
    ensure!(!error,"Circus history preprocessing Shift-JIS");Ok(bytes.into_owned())
}
fn find(text:&[u8],needle:&[u8])->Option<usize> {
    if needle.is_empty(){Some(0)}else{text.windows(needle.len()).position(|value|value==needle)}
}
fn budget(text:&[u8])->Result<()> {
    ensure!(text.len()<=crate::MAX_STRING_BYTES*4,"Circus history native substitution budget");Ok(())
}

pub(super) fn preprocess(text:&str,names:&[String;2],changed:bool)->Result<String> {
    let mut text=encoded(text)?;
    if let Some(end)=text.iter().position(|byte|*byte==0){text.truncate(end)}
    let names=[encoded(&names[0])?,encoded(&names[1])?];
    let mut at=0;
    let mut operations=0usize;
    let mut continue_after_re=false;
    //4012FA zeros both scratch1024-byte name-replacement strings once. A
    //subsequent incomplete @re1 therefore retains the preceding replacement.
    let mut replacement=Vec::new();
    let mut original_scratch=vec![0u8;1024];
    while at<text.len() {
        operations+=1;
        ensure!(operations<=crate::MAX_STRING_BYTES*16,"Circus cyclic history replacement");
        let rest=&text[at..];
        let skip_prefix=std::mem::take(&mut continue_after_re);
        if !skip_prefix&&(rest.starts_with(b"&f")||rest.starts_with(b"&n")) {
            let name=if rest.starts_with(b"&f"){&names[1]}else{&names[0]};
            text.splice(at..at+2,name.iter().copied());budget(&text)?;at=0;continue
        }
        if !skip_prefix&&rest.starts_with(b"@re1") {
            //401465 tests @n anywhere in the original remaining block; only
            //its absence makes a literal LF terminate the replacement line.
            let explicit_end=find(rest,b"@n").is_some();
            let original_start=at+4;
            let second=find(&text[original_start..],b"@re2").map(|n|original_start+n);
            let copied=&text[original_start..second.unwrap_or(text.len())];
            ensure!(copied.len()<1024,"Circus history native replacement scratch overflow");
            original_scratch[..copied.len()].copy_from_slice(copied);
            //4014B4 checks strlen before terminating the newly copied text.
            //Retain old scratch bytes: a shorter later block can therefore
            //observe the previous terminator when testing a trailing LF.
            if second.is_some() {
                let length=original_scratch.iter().position(|byte|*byte==0).context("Circus history native replacement terminator")?;
                if length>0&&original_scratch[length-1]==b'\n'{original_scratch[length-1]=0;}
            }
            original_scratch[copied.len()]=0;
            let original=original_scratch[..original_scratch.iter().position(|byte|*byte==0).unwrap_or(original_scratch.len())].to_vec();
            let mut tail=text.len();
            if let Some(second)=second {
                let mut cursor=second+4;
                replacement.clear();
                while cursor<text.len() {
                    if text[cursor]==b'\n'&&!explicit_end{cursor+=1;break}
                    if text[cursor..].starts_with(b"@n"){cursor+=2;break}
                    replacement.push(text[cursor]);cursor+=1;
                }
                tail=cursor;
            }
            text.drain(at..tail);
            if changed {
                if original.is_empty(){text=replacement.clone();}
                else {
                    //4015B8 repeatedly searches the whole resulting string,
                    //including the replacement. Keep its ordering; cyclic
                    //native input fails visibly instead of hanging a worker.
                    while let Some(start)=find(&text,&original) {
                        operations+=1;
                        ensure!(operations<=crate::MAX_STRING_BYTES*16,"Circus cyclic history name replacement");
                        text.splice(start..start+original.len(),replacement.iter().copied());budget(&text)?;
                    }
                }
            }
            //4016C2 rejoins the fixed-effect comparisons at the new start,
            //rather than re-running &name/@re1 checks in this iteration.
            at=0;continue_after_re=true;continue
        }
        let numeric=[b"@cmd".as_slice(),b"@d",b"@t",b"@muf",b"@mu",b"@se",b"@lse"].into_iter().find(|prefix|rest.starts_with(prefix));
        let remove=if let Some(prefix)=numeric {
            prefix.len()+rest[prefix.len()..].iter().take_while(|byte|byte.is_ascii_digit()).count()
        }else if [b"@w1".as_slice(),b"@w2",b"@w3",b"@w4",b"@w5",b"@ht",b"@hy",b"@b1",b"@b2",b"@b3",b"@b4"].iter().any(|prefix|rest.starts_with(prefix)) {3}
        else if [b"@wb1".as_slice(),b"@wb2",b"@wb3",b"@wb4",b"@wb5",b"@bo1",b"@bo2",b"@bo3",b"@bo4",b"@bi1",b"@bi2",b"@bi3",b"@bi4",b"@bt1",b"@bt2",b"@bt3",b"@bt4",b"@by1",b"@by2",b"@by3",b"@by4"].iter().any(|prefix|rest.starts_with(prefix)) {4}
        else {0};
        if remove!=0 {text.drain(at..at+remove);at=0;}else{at+=1;}
    }
    //401E85 handles only the first special dotted ruby. It matches CP932
    //byte lengths, generating two-byte middle dots up to the base's length;
    //ordinary reading strings and all later ruby entries are unchanged.
    if let Some(open)=find(&text,&[0x81,0x6f,0x81,0x45]) {
        let separator=find(&text,&[0x81,0x5e]).context("Circus history dotted ruby separator")?;
        ensure!(separator>=open+2,"Circus history dotted ruby order");
        let base=separator+2;
        let close=find(&text[base..],&[0x81,0x70]).map(|n|base+n).context("Circus history dotted ruby terminator")?;
        let reading_length=separator-(open+2);
        let base_length=close-base;
        if reading_length!=base_length {
            let dots=[0x81,0x45].repeat(base_length.div_ceil(2));
            text.splice(open+2..separator,dots);budget(&text)?;
        }
    }
    let (result,_,error)=SHIFT_JIS.decode(&text);
    ensure!(!error,"Circus history native preprocessing split encoded character");
    Ok(result.into_owned())
}
