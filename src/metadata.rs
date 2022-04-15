use std::{collections::HashMap, io::Write};

use nom::{branch::alt, bytes::complete::tag, character::streaming::char, multi::many1, IResult};
use nom_bibtex::Bibliography;

use crate::{
    author::{author, Author},
    paragraph::paragraph,
    r#abstract::{r#abstract, Abstract},
    space::space,
};

pub struct Metadata<'a> {
    authors: Option<Vec<Author<'a>>>,
    title: Option<&'a [u8]>,
    first_page: Option<&'a [u8]>,
    last_page: Option<&'a [u8]>,
    r#abstract: Option<Abstract<'a>>,
    keywords: Option<&'a [u8]>,
    section: Option<&'a [u8]>,
    number: Option<&'a [u8]>,
    semester: Option<&'a [u8]>,
    year: Option<&'a [u8]>,
}

impl<'a> Default for Metadata<'a> {
    fn default() -> Self {
        Self {
            authors: Default::default(),
            title: Default::default(),
            first_page: Default::default(),
            last_page: Default::default(),
            r#abstract: Default::default(),
            keywords: Default::default(),
            section: Default::default(),
            number: Default::default(),
            semester: Default::default(),
            year: Default::default(),
        }
    }
}

impl<'a> Metadata<'a> {
    pub fn wtite_to(
        &self,
        mut write: impl Write,
        bib: &HashMap<&[u8], &Bibliography>,
        date: chrono::DateTime<chrono::Utc>,
    ) -> std::io::Result<()> {
        fn escape(mut write: impl Write, mut text: &[u8]) -> std::io::Result<()> {
            loop {
                match text.iter().position(|b| *b == b'"') {
                    Some(pos) => {
                        write.write_all(&text[..pos])?;
                        write.write_all(b"\\\"")?;
                        text = &text[pos + 1..];
                    }
                    None => {
                        write.write_all(text)?;
                        break;
                    }
                }
            }
            Ok(())
        }

        write.write_all(b"---\n")?;
        if let Some(title) = self.title {
            write.write_all(b"title: \"")?;
            escape(&mut write, title)?;
            write.write_all(b"\"\n")?;
        }

        if let Some(r#abstract) = self.r#abstract.as_ref() {
            write.write_all(b"description: \"")?;
            let mut buf = Vec::new();
            r#abstract.write_to(&mut buf, bib, crate::r#abstract::Format::PlainText)?;
            if buf.len() > 143 {
                buf.truncate(140);
                buf.push(b'.');
                buf.push(b'.');
                buf.push(b'.');
            }
            escape(&mut write, buf.as_slice())?;
            write.write_all(b"\"\n")?;
        }

        write.write_all(b"date: ")?;
        write.write_all(format!("{}", date.format("%+")).as_bytes())?;
        write.write_all(b"\n")?;

        if let Some(authors) = self.authors.as_ref() {
            write.write_all(b"authors:")?;
            for author in authors {
                write.write_all(b"\n- given: ")?;
                write.write_all(author.given)?;
                write.write_all(b"\n  family: ")?;
                write.write_all(author.family)?;
            }
            write.write_all(b"\n")?;
        }

        if let Some(keywords) = self.keywords {
            write.write_all(b"tags:")?;
            for kw in String::from_utf8_lossy(keywords).split(".") {
                let kw = kw.trim();
                if !kw.is_empty() {
                    write.write_all(b"\n- ")?;
                    write.write_all(kw.as_bytes())?;
                }
            }
            write.write_all(b"\n")?;
        }

        if let Some(first_page) = self.first_page {
            if let Some(last_page) = self.last_page {
                write.write_all(b"pages: [")?;
                write.write_all(first_page)?;
                write.write_all(b", ")?;
                write.write_all(last_page)?;
                write.write_all(b"]\n")?;
            }
        }

        if let Some(section) = self.section {
            write.write_all(b"section: \"")?;
            escape(&mut write, section)?;
            write.write_all(b"\"\n")?;
        }

        if let Some(number) = self.number {
            write.write_all(b"series: [n")?;
            escape(&mut write, number)?;
            write.write_all(b"]\n")?;

            write.write_all(b"number: ")?;
            escape(&mut write, number)?;
            write.write_all(b"\n")?;
        }

        if let Some(semester) = self.semester {
            write.write_all(b"semester: ")?;
            escape(&mut write, semester)?;
            write.write_all(b"\n")?;
        }

        if let Some(year) = self.year {
            write.write_all(b"year: ")?;
            escape(&mut write, String::from_utf8_lossy(year).trim().as_bytes())?;
            write.write_all(b"\n")?;
        }

        write.write_all(b"---\n\n")?;

        if let Some(r#abstract) = self.r#abstract.as_ref() {
            write.write_all(b"**Resumo:** ")?;
            r#abstract.write_to(&mut write, bib, crate::r#abstract::Format::Markdown)?;
            write.write_all(b"\n\n")?;
        }

        if let Some(keywords) = self.keywords {
            write.write_all(b"**Palavras-chave:** ")?;
            write.write_all(keywords)?;
            write.write_all(b"\n")?;
        }

        Ok(())
    }
}

fn divisor(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = space(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = space(input)?;
    Ok((input, ()))
}

pub fn metadata(input: &[u8]) -> IResult<&[u8], Metadata> {
    let mut key = alt::<&[u8], _, nom::error::Error<&[u8]>, _>((
        tag("authors"),
        tag("title"),
        tag("first_page"),
        tag("last_page"),
        tag("abstract"),
        tag("keywords"),
        tag("section"),
        tag("number"),
        tag("semester"),
        tag("year"),
        tag("\\par"),
    ));

    let mut input = input;
    let mut metadata = Metadata::default();

    loop {
        let (inp, _) = space(input)?;
        let (inp, key) = match key(inp) {
            Ok(ok) => ok,
            Err(_) => break,
        };

        if key == b"\\par" {
            input = inp;
            continue;
        }

        let (inp, _) = divisor(inp)?;

        input = match key {
            b"authors" => {
                let (inp, authors) = many1(author)(inp)?;
                let (inp, _) = paragraph(inp)?;
                metadata.authors = Some(authors);
                inp
            }
            b"title" => {
                let (inp, title) = paragraph(inp)?;
                metadata.title = Some(title);
                inp
            }
            b"first_page" => {
                let (inp, first_page) = paragraph(inp)?;
                metadata.first_page = Some(first_page);
                inp
            }
            b"last_page" => {
                let (inp, last_page) = paragraph(inp)?;
                metadata.last_page = Some(last_page);
                inp
            }
            b"abstract" => {
                let (inp, summary) = r#abstract(inp)?;
                let (inp, _) = paragraph(inp)?;
                metadata.r#abstract = Some(summary);
                inp
            }
            b"keywords" => {
                let (inp, keywords) = paragraph(inp)?;
                metadata.keywords = Some(keywords);
                inp
            }
            b"section" => {
                let (inp, section) = paragraph(inp)?;
                metadata.section = Some(section);
                inp
            }
            b"number" => {
                let (inp, number) = paragraph(inp)?;
                metadata.number = Some(number);
                inp
            }
            b"semester" => {
                let (inp, semester) = paragraph(inp)?;
                metadata.semester = Some(semester);
                inp
            }
            b"year" => {
                let (inp, year) = paragraph(inp)?;
                metadata.year = Some(year);
                inp
            }
            _ => unreachable!(),
        }
    }

    Ok((input, metadata))
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn euclides_da_cunha() {
        const INPUT_STR: &str = r#" authors=given> Aurora Almeida de Miranda, family> Leão\par title=Euclides da Cunha atualizado no sertão da teledramaturgia\par first_page=15\par last_page=29\par abstract=O objeto deste artigo é a série \textit {Onde nascem os fortes} (TV Globo, 2018), escrita para exibição em canal aberto de televisão, em ano eleitoral e filmada no cariri paraibano. A partir do título e da ambiência, percebemos uma configuração que remete ao livro \textit {Os sertões} \citeyear {EcCUNHA1902sertoes}. Objetiva-se perscrutar como o conceito de sertão é trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no início do século XX permanecem neste século XXI com impressionante atualidade. Ademais, o território sertanejo revela-se como poderoso cronotopo \cite {EcBAKHTIN2003Estetica}, em forte simetria com a linha abissal da Sociologia das Ausências \cite {EcSANTOS2004Para}. Elege-se o capítulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na técnica da minutagem, através da qual analisa-se as estratégias de construção narrativa \cite {EcMOTTA2013analise}, bem como os procedimentos de elaboração do roteiro \cite {EcMACIEL2017poder}. Conclui-se que o episódio inaugural figura como síntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percepções destacadas por Euclides da Cunha aparecem e dão pistas de como o roteiro prosseguirá, embora trazendo ressignificações para o espaço sertanejo e os personagens que o habitam. \par keywords=Onde nascem os fortes. Euclides da Cunha. Sertão. Teledramaturgia. Narrativa.\par section=Dossiê História dos Sertões: espaços, sentidos e saberes\par number=5\par semester=1\par year=2022"#;

        let (input, _metadata) = metadata(INPUT_STR.as_bytes()).unwrap();

        assert!(input.is_empty());
    }
}
