//! Unicode full-width conversion for text input and legacy script strings.
//! Unrelated compatibility characters and already-wide text remain unchanged.
const KANA:&str="。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";
fn voiced(c:char,semi:bool)->Option<char>{
    if semi{return match c{'ハ'=>Some('パ'),'ヒ'=>Some('ピ'),'フ'=>Some('プ'),'ヘ'=>Some('ペ'),'ホ'=>Some('ポ'),_=>None};}
    match c {
        'ウ'=>Some('ヴ'),'ワ'=>Some('ヷ'),'ヰ'=>Some('ヸ'),'ヱ'=>Some('ヹ'),'ヲ'=>Some('ヺ'),
        'カ'|'キ'|'ク'|'ケ'|'コ'|'サ'|'シ'|'ス'|'セ'|'ソ'|'タ'|'チ'|'ツ'|'テ'|'ト'|'ハ'|'ヒ'|'フ'|'ヘ'|'ホ'=>char::from_u32(c as u32+1),
        _=>None,
    }
}
pub fn to_full_width(text:&str)->String{
    let kana=KANA.chars().collect::<Vec<_>>();
    let mut out=String::with_capacity(text.len());let mut input=text.chars().peekable();
    while let Some(c)=input.next(){
        let mapped=match c{' '=>'\u{3000}','!'..='~'=>char::from_u32(c as u32+0xfee0).unwrap(),'\u{ff61}'..='\u{ff9f}'=>kana[c as usize-0xff61],_=>c};
        if ('\u{ff66}'..='\u{ff9d}').contains(&c){
            if let Some(&mark @ ('\u{ff9e}'|'\u{ff9f}'))=input.peek(){
                if let Some(composed)=voiced(mapped,mark=='\u{ff9f}') {out.push(composed);input.next();continue;}
            }
        }
        out.push(mapped);
    }
    out
}
