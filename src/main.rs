use std::{fs::File, io::Read, time::SystemTime};

use nom_bibtex::Bibtex;

mod r#abstract;
mod author;
mod metadata;
mod paragraph;
mod space;

fn main() {
    let mut args = std::env::args().fuse().skip(1);
    let metadata = args.next().expect("valid metadata file");

    let mut metadata = File::open(metadata).unwrap();
    let mut buf = Vec::new();

    metadata.read_to_end(&mut buf).unwrap();

    let (input, metadata) = crate::metadata::metadata(buf.as_slice()).unwrap();

    assert!(input.is_empty());

    let bib = args
        .next()
        .map(|path| {
            let mut file = File::open(path).unwrap();
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).unwrap();

            String::from_utf8(buf).unwrap()
        })
        .unwrap_or_default();

    let bib = Bibtex::parse(&bib).expect("valid bibliographies");

    let bib = bib
        .bibliographies()
        .iter()
        .map(|b| (b.citation_key().as_bytes(), b))
        .collect();

    metadata
        .wtite_to(std::io::stdout(), &bib, SystemTime::now().into())
        .unwrap();
}
