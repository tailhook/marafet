pub fn join<S1, S2, I>(mut iter: I, sep: S2) -> String
    where S1:AsRef<str>, S2:AsRef<str>, I:Iterator<Item=S1>
{
    let mut buf = String::new();
    match iter.next() {
        Some(x) => buf.push_str(x.as_ref()),
        None => {}
    }
    for i in iter {
        buf.push_str(sep.as_ref());
        buf.push_str(i.as_ref());
    }
    return buf;
}
