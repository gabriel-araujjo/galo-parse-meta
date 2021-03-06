use std::{collections::HashMap, io::Write, borrow::Cow};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::char,
    error::ErrorKind,
    multi::many0,
    sequence::delimited,
    IResult,
};
use nom_bibtex::Bibliography;

use crate::space::space;

#[derive(Debug, PartialEq, Clone, Copy)]
enum AbstractPart<'a> {
    Text(&'a [u8]),
    Textit(&'a [u8]),
    Citeyear(&'a [u8]),
    Cite(&'a [u8]),
}

pub struct Abstract<'a> {
    parts: Vec<AbstractPart<'a>>,
}

pub enum Format {
    Markdown,
    PlainText,
}

impl Format {
    fn italic(&self, mut write: impl Write, text: &[u8]) -> std::io::Result<()> {
        match self {
            Format::Markdown => {
                write.write_all(b"_")?;
                write.write_all(text)?;
                write.write_all(b"_")
            }
            Format::PlainText => write.write_all(text),
        }
    }
}

impl<'a> Abstract<'a> {
    pub fn write_to(
        &self,
        mut write: impl Write,
        bib: &HashMap<&[u8], &Bibliography>,
        format: Format,
    ) -> std::io::Result<()> {
        for part in self.parts.iter().copied() {
            match part {
                AbstractPart::Text(text) => write.write_all(text)?,
                AbstractPart::Textit(text) => {
                    format.italic(&mut write, text)?;
                }
                AbstractPart::Citeyear(key) => {
                    let bib = match bib.get(key) {
                        Some(bib) => bib,
                        None => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("bibliography not found: {}", String::from_utf8_lossy(key)),
                            ))
                        }
                    };

                    let year = bib
                        .tags()
                        .iter()
                        .find_map(|(k, v)| if k == "year" { Some(v.as_str()) } else { None })
                        .unwrap_or("_s.d._");

                    write.write_all(b"(")?;
                    write.write_all(year.as_bytes())?;
                    write.write_all(b")")?;
                }
                AbstractPart::Cite(key) => {
                    let bib = match bib.get(key) {
                        Some(bib) => bib,
                        None => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("bibliography not found: {}", String::from_utf8_lossy(key)),
                            ))
                        }
                    };

                    let year = bib
                        .tags()
                        .iter()
                        .find_map(|(k, v)| if k == "year" { Some(v.as_str()) } else { None })
                        .unwrap_or("_s.d._");

                    let author = bib
                        .tags()
                        .iter()
                        .find_map(|(k, v)| {
                            if k == "author" {
                                let s: Vec<_> = v.split(" AND ")
                                    .map(|a| a.split(',').next().unwrap())
                                    .collect();
                                
                                if s.len() > 3 {
                                    let mut s = s[0].to_owned();
                                    s += ", _et al._";
                                    Some(Cow::Owned(s))
                                } else if s.len() > 1 {
                                    Some(Cow::Owned(s.join("; ")))
                                } else {
                                    Some(Cow::Borrowed(s[1]))
                                }
                            } else {
                                None
                            }
                        })
                        .unwrap_or(
                            bib.tags()
                                .iter()
                                .find_map(|(k, v)| {
                                    if k == "title" {
                                        Some(Cow::Borrowed(v.as_str().split(' ').next().unwrap()))
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(Cow::Borrowed("")),
                        );

                    write.write_all(b"(")?;
                    write.write_all(author.trim().to_uppercase().as_bytes())?;
                    write.write_all(b", ")?;
                    write.write_all(year.trim().as_bytes())?;
                    write.write_all(b")")?;
                }
            }
        }

        Ok(())
    }
}

fn block(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let braced = delimited(char('{'), is_not(&b"}"[..]), char('}'));
    let not_braced = is_not(&b" \t\r\n"[..]);

    alt((braced, not_braced))(input)
}

fn command(input: &[u8]) -> IResult<&[u8], AbstractPart> {
    let (input, _) = space(input)?;
    let original_input = input;
    let (input, _) = tag("\\")(input)?;

    let (input, command) = alt((tag("textit"), tag("citeyear"), tag("cite")))(input)?;

    let (input, _) = space(input)?;

    let (input, arg) = block(input)?;

    let part = match command {
        b"textit" => AbstractPart::Textit(arg),
        b"citeyear" => AbstractPart::Citeyear(arg),
        b"cite" => AbstractPart::Cite(arg),
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                original_input,
                ErrorKind::Satisfy,
            )))
        }
    };

    Ok((input, part))
}

fn text(input: &[u8]) -> IResult<&[u8], AbstractPart> {
    let (input, text) = is_not(&b"\\"[..])(input)?;

    Ok((input, AbstractPart::Text(text)))
}

pub fn r#abstract(input: &[u8]) -> IResult<&[u8], Abstract> {
    let part = alt((text, command));
    let (input, parts) = many0(part)(input)?;
    Ok((input, Abstract { parts }))
}

#[cfg(test)]
mod test {

    use std::borrow::{Borrow, Cow};

    use nom_bibtex::Bibtex;

    use super::*;

    #[test]
    fn simple_abstract() {
        let input = r#"O objeto deste artigo ?? a s??rie \textit {Onde nascem os fortes} (TV Globo, 2018), escrita para exibi????o em canal aberto de televis??o, em ano eleitoral e filmada no cariri paraibano. A partir do t??tulo e da ambi??ncia, percebemos uma configura????o que remete ao livro \textit {Os sert??es} \citeyear {EcCUNHA1902sertoes}. Objetiva-se perscrutar como o conceito de sert??o ?? trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no in??cio do s??culo XX permanecem neste s??culo XXI com impressionante atualidade. Ademais, o territ??rio sertanejo revela-se como poderoso cronotopo \cite {EcBAKHTIN2003Estetica}, em forte simetria com a linha abissal da Sociologia das Aus??ncias \cite {EcSANTOS2004Para}. Elege-se o cap??tulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na t??cnica da minutagem, atrav??s da qual analisa-se as estrat??gias de constru????o narrativa \cite {EcMOTTA2013analise}, bem como os procedimentos de elabora????o do roteiro \cite {EcMACIEL2017poder}. Conclui-se que o epis??dio inaugural figura como s??ntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percep????es destacadas por Euclides da Cunha aparecem e d??o pistas de como o roteiro prosseguir??, embora trazendo ressignifica????es para o espa??o sertanejo e os personagens que o habitam."#;

        let (input, abs) = r#abstract(input.as_bytes()).unwrap();

        assert!(input.is_empty());

        assert_eq!(
            abs.parts,
            vec![
                AbstractPart::Text(r#"O objeto deste artigo ?? a s??rie "#.as_bytes()),
                AbstractPart::Textit(r#"Onde nascem os fortes"#.as_bytes()),
                AbstractPart::Text(r#" (TV Globo, 2018), escrita para exibi????o em canal aberto de televis??o, em ano eleitoral e filmada no cariri paraibano. A partir do t??tulo e da ambi??ncia, percebemos uma configura????o que remete ao livro "#.as_bytes()),
                AbstractPart::Textit(r#"Os sert??es"#.as_bytes()),
                AbstractPart::Text(" ".as_bytes()),
                AbstractPart::Citeyear(r#"EcCUNHA1902sertoes"#.as_bytes()),
                AbstractPart::Text(r#". Objetiva-se perscrutar como o conceito de sert??o ?? trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no in??cio do s??culo XX permanecem neste s??culo XXI com impressionante atualidade. Ademais, o territ??rio sertanejo revela-se como poderoso cronotopo "#.as_bytes()),
                AbstractPart::Cite(r#"EcBAKHTIN2003Estetica"#.as_bytes()),
                AbstractPart::Text(r#", em forte simetria com a linha abissal da Sociologia das Aus??ncias "#.as_bytes()),
                AbstractPart::Cite(r#"EcSANTOS2004Para"#.as_bytes()),
                AbstractPart::Text(r#". Elege-se o cap??tulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na t??cnica da minutagem, atrav??s da qual analisa-se as estrat??gias de constru????o narrativa "#.as_bytes()),
                AbstractPart::Cite(r#"EcMOTTA2013analise"#.as_bytes()),
                AbstractPart::Text(r#", bem como os procedimentos de elabora????o do roteiro "#.as_bytes()),
                AbstractPart::Cite(r#"EcMACIEL2017poder"#.as_bytes()),
                AbstractPart::Text(r#". Conclui-se que o epis??dio inaugural figura como s??ntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percep????es destacadas por Euclides da Cunha aparecem e d??o pistas de como o roteiro prosseguir??, embora trazendo ressignifica????es para o espa??o sertanejo e os personagens que o habitam."#.as_bytes()),
            ],
        );
    }

    #[test]
    fn markdown() {
        let input = r#"O objeto deste artigo ?? a s??rie \textit {Onde nascem os fortes} (TV Globo, 2018), escrita para exibi????o em canal aberto de televis??o, em ano eleitoral e filmada no cariri paraibano. A partir do t??tulo e da ambi??ncia, percebemos uma configura????o que remete ao livro \textit {Os sert??es} \citeyear {EcCUNHA1902sertoes}. Objetiva-se perscrutar como o conceito de sert??o ?? trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no in??cio do s??culo XX permanecem neste s??culo XXI com impressionante atualidade. Ademais, o territ??rio sertanejo revela-se como poderoso cronotopo \cite {EcBAKHTIN2003Estetica}, em forte simetria com a linha abissal da Sociologia das Aus??ncias \cite {EcSANTOS2004Para}. Elege-se o cap??tulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na t??cnica da minutagem, atrav??s da qual analisa-se as estrat??gias de constru????o narrativa \cite {EcMOTTA2013analise}, bem como os procedimentos de elabora????o do roteiro \cite {EcMACIEL2017poder}. Conclui-se que o epis??dio inaugural figura como s??ntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percep????es destacadas por Euclides da Cunha aparecem e d??o pistas de como o roteiro prosseguir??, embora trazendo ressignifica????es para o espa??o sertanejo e os personagens que o habitam."#;

        let bibliography = r#"
        @book{EcALBUQUERQUE2013Nordestino,
            author    = {Albuquerque Jr., D. M.},
            title     = {Nordestino},
            subtitle  = {uma inven????o do falo; uma Hist??ria do g??nero masculino},
            location  = {Macei??},
            publisher = {Editora Catavento},
            year      = {2013}
          }
          @book{EcALBUQUERQUEJR2001invencao,
            author    = {Albuquerque Jr., D. M.},
            title     = {A inven????o do Nordeste e outras artes},
            location  = {S??o Paulo},
            publisher = {Cortez},
            year      = {2001}
          }
          @book{EcBAKHTIN2003Estetica,
            author    = {Bakhtin, M.},
            title     = {Est??tica da cria????o verbal},
            location  = {S??o Paulo},
            publisher = {Martins Fontes},
            year      = {2003}
          }
          @book{EcCUNHA1902sertoes,
            author    = {Cunha, E.},
            title     = {Os sert??es},
            location  = {S??o Paulo},
            publisher = {Editora Martin Claret},
            year      = {1902}
          }
          @book{EcFERNANDES1977sociologia,
            author    = {Fernandes, F.},
            title     = {A sociologia no Brasil},
            location  = {Petr??polis},
            publisher = {Vozes},
            year      = {1977}
          }
          @book{EcMACHADO2012Tecnologias,
            author    = {Machado, J.},
            title     = {Tecnologias do imagin??rio},
            location  = {Porto Alegre},
            publisher = {Editora Sulina},
            year      = {2012}
          }
          @book{EcMACHADO2010que,
            author    = {Machado, J.},
            title     = {O que pesquisar quer dizer},
            subtitle  = {como fazer textos acad??micos sem medo da ABNT e da CAPES ??? An??lise Discursiva de Imagin??rios (ADI)},
            location  = {Porto Alegre},
            publisher = {Editora Sulina},
            year      = {2010}
          }
          @book{EcMACIEL2017poder,
            author    = {Maciel, L. C.},
            title     = {O poder do cl??max},
            subtitle  = {fundamentos do roteiro de cinema e TV},
            location  = {S??o Paulo},
            publisher = {Editora Giostri},
            year      = {2017}
          }
          @book{EcMENESES2andSANTOS2009Epistemologias,
            author    = {Meneses, M. P. AND Santos, B. S.},
            title     = {Epistemologias do Sul},
            location  = {Coimbra},
            publisher = {Edi????es Almedina},
            year      = {2009}
          }
          @book{EcMOTTA2013analise,
            author    = {Motta, L. G.},
            title     = {A an??lise cr??tica da narrativa},
            location  = {Bras??lia},
            publisher = {EdUnB},
            year      = {2013}
          }
          @inproceedings{EcMOTTERTelenovela,
            author     = {Motter, Maria de Lourdes},
            title      = {Telenovela},
            subtitle   = {reflexo e refra????o na arte do cotidiano},
            eventtitle = {Congresso Brasileiro de Ci??ncias da Comunica????o},
            number     = {21},
            venue      = {Recife},
            volume     = {21},
            eventyear  = {1998},
            location   = {Recife},
            year       = {1998},
            url        = {http://www.portcom.intercom.org.br/pdfs/de14671ff94329deb4d1756ec2696184.PDF}
          }
          @article{EcREZENDE2001sertoes,
            author   = {Rezende, M. J.},
            title    = {Os sert??es e os (des)caminhos da mudan??a social no Brasil},
            location = {S??o Paulo},
            journal  = {Tempo Social: Revista de Sociologia da USP},
            volume   = {13},
            number   = {2},
            year     = {2001},
            pages    = {201--226}
          }
          @incollection{EcSANTOS2004Para,
            author     = {Santos, B. S.},
            title      = {Para uma sociologia das aus??ncias e uma sociologia das emerg??ncias},
            booktitle  = {Conhecimento prudente para uma vida decente},
            editor     = {Santos, B. S.},
            editortype = {organizer},
            location   = {S??o Paulo},
            publisher  = {Cortez},
            year       = {2004}
          }
          @incollection{EcSANTOS2009ParaAlem,
            author     = {Santos, B. S.},
            title      = {Para al??m do Pensamento Abissal},
            subtitle   = {das linhas globais a uma ecologia de saberes },
            booktitle  = {Epistemologias do Sul},
            editor     = {Santos, B. S. AND Maria Paula Meneses},
            editortype = {organizer},
            location   = {Coimbra},
            publisher  = {Edi????es Almedina},
            year       = {2009}
          }
          @article{EcTELES2009Lugar,
            author   = {Teles, G. M.},
            title    = {O lu(g)ar dos sert??es},
            location = {Juiz de Fora},
            journal  = {Revista Verbo de Minas},
            volume   = {8},
            number   = {16},
            year     = {2009}
          }
          @incollection{EcVASCONCELLOS2014Entre,
            author     = {Vasconcellos, C. P. V.},
            title      = {Entre representa????es e estere??tipos},
            subtitle   = {o sert??o na constru????o da brasilidade},
            booktitle  = {Culturas dos sert??es},
            editor     = {Pereira, A.},
            editortype = {organizer},
            location   = {Salvador},
            publisher  = {EdUFBA},
            year       = {2014}
          }
          @book{EcWOLTON1996Elogio,
            title     = {Elogio do Grande P??blico},
            subtitle  = {Uma teoria cr??tica da televis??o},
            author    = {Wolton, D.},
            year      = {1996},
            publisher = {??tica},
            location  = {S??o Paulo}
          }
          @article{EcC??DIMA2001Proto,
            author   = {C??dima, Francisco Rui},
            title    = {Proto e p??s-televis??o. Adorno, Bourdieu e os outros???ou na pista da ??qualimetria??},
            location = {Lisboa},
            journal  = {Revista de Comunica????o e Linguagens},
            year     = {2001},
            number   = {30}
          }
          @book{EcSCOLARIHipermediaciones,
            author    = {Scolari, Carlos},
            title     = {Hipermediaciones},
            subtitle  = {Elementos para una teor??a de la comunicaci??n digital interactiva},
            publisher = {Gedisa},
            location  = {Barcelona},
            year      = {2008}
          }
          @book{EcARONCHI2015G??neros,
            author    = {de Souza, Jose Carlos Aronchi},
            title     = {Generos e Formatos na Televisao Brasileira},
            publisher = {Summus Editorial},
            location  = {S??o Paulo},
            year      = {2015}
          }
          @article{EcSILVA2017Aspectos,
            title   = {Aspectos do imagin??rio e da comunica????o em Grande Sert??o: Veredas},
            number  = {40},
            journal = {Intexto},
            author  = {Silva, Gustavo Castro},
            year    = {2017},
            month   = {8},
            pages   = {96--113}
          }
          @article{EcSANTOS2018Comunica????o,
            author  = {Santos, B. S.},
            title   = {A Comunica????o sob o olhar de Boaventura de Sousa Santos. [Entrevista concedida a] Eloisa Loose},
            journal = {A????o Midi??tica --- Estudos em Comunica????o, Sociedade e Cultura.},
            number  = {16},
            year    = {2018},
            pages   = {138--150}
          }          
        "#;

        let bib = Bibtex::parse(bibliography).unwrap();
        let bib: HashMap<_, _> = bib
            .bibliographies()
            .iter()
            .map(|b| (b.citation_key().as_bytes(), b))
            .collect();

        let (_, abs) = r#abstract(input.as_bytes()).unwrap();

        let mut output = Vec::new();

        abs.write_to(&mut output, &bib, Format::Markdown).unwrap();

        let s = String::from_utf8_lossy(output.as_slice());

        assert_eq!(
            <Cow<'_, str> as Borrow<str>>::borrow(&s),
            r#"O objeto deste artigo ?? a s??rie _Onde nascem os fortes_ (TV Globo, 2018), escrita para exibi????o em canal aberto de televis??o, em ano eleitoral e filmada no cariri paraibano. A partir do t??tulo e da ambi??ncia, percebemos uma configura????o que remete ao livro _Os sert??es_ (1902). Objetiva-se perscrutar como o conceito de sert??o ?? trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no in??cio do s??culo XX permanecem neste s??culo XXI com impressionante atualidade. Ademais, o territ??rio sertanejo revela-se como poderoso cronotopo (BAKHTIN, 2003), em forte simetria com a linha abissal da Sociologia das Aus??ncias (SANTOS, 2004). Elege-se o cap??tulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na t??cnica da minutagem, atrav??s da qual analisa-se as estrat??gias de constru????o narrativa (MOTTA, 2013), bem como os procedimentos de elabora????o do roteiro (MACIEL, 2017). Conclui-se que o epis??dio inaugural figura como s??ntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percep????es destacadas por Euclides da Cunha aparecem e d??o pistas de como o roteiro prosseguir??, embora trazendo ressignifica????es para o espa??o sertanejo e os personagens que o habitam."#
        )
    }
}
