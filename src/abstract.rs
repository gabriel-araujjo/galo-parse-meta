use std::{collections::HashMap, io::Write};

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
                                let s = v.as_str().split(',').next().unwrap();
                                Some(s)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(
                            bib.tags()
                                .iter()
                                .find_map(|(k, v)| {
                                    if k == "title" {
                                        Some(v.as_str().split(' ').next().unwrap())
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(""),
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
        let input = r#"O objeto deste artigo é a série \textit {Onde nascem os fortes} (TV Globo, 2018), escrita para exibição em canal aberto de televisão, em ano eleitoral e filmada no cariri paraibano. A partir do título e da ambiência, percebemos uma configuração que remete ao livro \textit {Os sertões} \citeyear {EcCUNHA1902sertoes}. Objetiva-se perscrutar como o conceito de sertão é trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no início do século XX permanecem neste século XXI com impressionante atualidade. Ademais, o território sertanejo revela-se como poderoso cronotopo \cite {EcBAKHTIN2003Estetica}, em forte simetria com a linha abissal da Sociologia das Ausências \cite {EcSANTOS2004Para}. Elege-se o capítulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na técnica da minutagem, através da qual analisa-se as estratégias de construção narrativa \cite {EcMOTTA2013analise}, bem como os procedimentos de elaboração do roteiro \cite {EcMACIEL2017poder}. Conclui-se que o episódio inaugural figura como síntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percepções destacadas por Euclides da Cunha aparecem e dão pistas de como o roteiro prosseguirá, embora trazendo ressignificações para o espaço sertanejo e os personagens que o habitam."#;

        let (input, abs) = r#abstract(input.as_bytes()).unwrap();

        assert!(input.is_empty());

        assert_eq!(
            abs.parts,
            vec![
                AbstractPart::Text(r#"O objeto deste artigo é a série "#.as_bytes()),
                AbstractPart::Textit(r#"Onde nascem os fortes"#.as_bytes()),
                AbstractPart::Text(r#" (TV Globo, 2018), escrita para exibição em canal aberto de televisão, em ano eleitoral e filmada no cariri paraibano. A partir do título e da ambiência, percebemos uma configuração que remete ao livro "#.as_bytes()),
                AbstractPart::Textit(r#"Os sertões"#.as_bytes()),
                AbstractPart::Text(" ".as_bytes()),
                AbstractPart::Citeyear(r#"EcCUNHA1902sertoes"#.as_bytes()),
                AbstractPart::Text(r#". Objetiva-se perscrutar como o conceito de sertão é trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no início do século XX permanecem neste século XXI com impressionante atualidade. Ademais, o território sertanejo revela-se como poderoso cronotopo "#.as_bytes()),
                AbstractPart::Cite(r#"EcBAKHTIN2003Estetica"#.as_bytes()),
                AbstractPart::Text(r#", em forte simetria com a linha abissal da Sociologia das Ausências "#.as_bytes()),
                AbstractPart::Cite(r#"EcSANTOS2004Para"#.as_bytes()),
                AbstractPart::Text(r#". Elege-se o capítulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na técnica da minutagem, através da qual analisa-se as estratégias de construção narrativa "#.as_bytes()),
                AbstractPart::Cite(r#"EcMOTTA2013analise"#.as_bytes()),
                AbstractPart::Text(r#", bem como os procedimentos de elaboração do roteiro "#.as_bytes()),
                AbstractPart::Cite(r#"EcMACIEL2017poder"#.as_bytes()),
                AbstractPart::Text(r#". Conclui-se que o episódio inaugural figura como síntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percepções destacadas por Euclides da Cunha aparecem e dão pistas de como o roteiro prosseguirá, embora trazendo ressignificações para o espaço sertanejo e os personagens que o habitam."#.as_bytes()),
            ],
        );
    }

    #[test]
    fn markdown() {
        let input = r#"O objeto deste artigo é a série \textit {Onde nascem os fortes} (TV Globo, 2018), escrita para exibição em canal aberto de televisão, em ano eleitoral e filmada no cariri paraibano. A partir do título e da ambiência, percebemos uma configuração que remete ao livro \textit {Os sertões} \citeyear {EcCUNHA1902sertoes}. Objetiva-se perscrutar como o conceito de sertão é trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no início do século XX permanecem neste século XXI com impressionante atualidade. Ademais, o território sertanejo revela-se como poderoso cronotopo \cite {EcBAKHTIN2003Estetica}, em forte simetria com a linha abissal da Sociologia das Ausências \cite {EcSANTOS2004Para}. Elege-se o capítulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na técnica da minutagem, através da qual analisa-se as estratégias de construção narrativa \cite {EcMOTTA2013analise}, bem como os procedimentos de elaboração do roteiro \cite {EcMACIEL2017poder}. Conclui-se que o episódio inaugural figura como síntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percepções destacadas por Euclides da Cunha aparecem e dão pistas de como o roteiro prosseguirá, embora trazendo ressignificações para o espaço sertanejo e os personagens que o habitam."#;

        let bibliography = r#"
        @book{EcALBUQUERQUE2013Nordestino,
            author    = {Albuquerque Jr., D. M.},
            title     = {Nordestino},
            subtitle  = {uma invenção do falo; uma História do gênero masculino},
            location  = {Maceió},
            publisher = {Editora Catavento},
            year      = {2013}
          }
          @book{EcALBUQUERQUEJR2001invencao,
            author    = {Albuquerque Jr., D. M.},
            title     = {A invenção do Nordeste e outras artes},
            location  = {São Paulo},
            publisher = {Cortez},
            year      = {2001}
          }
          @book{EcBAKHTIN2003Estetica,
            author    = {Bakhtin, M.},
            title     = {Estética da criação verbal},
            location  = {São Paulo},
            publisher = {Martins Fontes},
            year      = {2003}
          }
          @book{EcCUNHA1902sertoes,
            author    = {Cunha, E.},
            title     = {Os sertões},
            location  = {São Paulo},
            publisher = {Editora Martin Claret},
            year      = {1902}
          }
          @book{EcFERNANDES1977sociologia,
            author    = {Fernandes, F.},
            title     = {A sociologia no Brasil},
            location  = {Petrópolis},
            publisher = {Vozes},
            year      = {1977}
          }
          @book{EcMACHADO2012Tecnologias,
            author    = {Machado, J.},
            title     = {Tecnologias do imaginário},
            location  = {Porto Alegre},
            publisher = {Editora Sulina},
            year      = {2012}
          }
          @book{EcMACHADO2010que,
            author    = {Machado, J.},
            title     = {O que pesquisar quer dizer},
            subtitle  = {como fazer textos acadêmicos sem medo da ABNT e da CAPES – Análise Discursiva de Imaginários (ADI)},
            location  = {Porto Alegre},
            publisher = {Editora Sulina},
            year      = {2010}
          }
          @book{EcMACIEL2017poder,
            author    = {Maciel, L. C.},
            title     = {O poder do clímax},
            subtitle  = {fundamentos do roteiro de cinema e TV},
            location  = {São Paulo},
            publisher = {Editora Giostri},
            year      = {2017}
          }
          @book{EcMENESES2andSANTOS2009Epistemologias,
            author    = {Meneses, M. P. AND Santos, B. S.},
            title     = {Epistemologias do Sul},
            location  = {Coimbra},
            publisher = {Edições Almedina},
            year      = {2009}
          }
          @book{EcMOTTA2013analise,
            author    = {Motta, L. G.},
            title     = {A análise crítica da narrativa},
            location  = {Brasília},
            publisher = {EdUnB},
            year      = {2013}
          }
          @inproceedings{EcMOTTERTelenovela,
            author     = {Motter, Maria de Lourdes},
            title      = {Telenovela},
            subtitle   = {reflexo e refração na arte do cotidiano},
            eventtitle = {Congresso Brasileiro de Ciências da Comunicação},
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
            title    = {Os sertões e os (des)caminhos da mudança social no Brasil},
            location = {São Paulo},
            journal  = {Tempo Social: Revista de Sociologia da USP},
            volume   = {13},
            number   = {2},
            year     = {2001},
            pages    = {201--226}
          }
          @incollection{EcSANTOS2004Para,
            author     = {Santos, B. S.},
            title      = {Para uma sociologia das ausências e uma sociologia das emergências},
            booktitle  = {Conhecimento prudente para uma vida decente},
            editor     = {Santos, B. S.},
            editortype = {organizer},
            location   = {São Paulo},
            publisher  = {Cortez},
            year       = {2004}
          }
          @incollection{EcSANTOS2009ParaAlem,
            author     = {Santos, B. S.},
            title      = {Para além do Pensamento Abissal},
            subtitle   = {das linhas globais a uma ecologia de saberes },
            booktitle  = {Epistemologias do Sul},
            editor     = {Santos, B. S. AND Maria Paula Meneses},
            editortype = {organizer},
            location   = {Coimbra},
            publisher  = {Edições Almedina},
            year       = {2009}
          }
          @article{EcTELES2009Lugar,
            author   = {Teles, G. M.},
            title    = {O lu(g)ar dos sertões},
            location = {Juiz de Fora},
            journal  = {Revista Verbo de Minas},
            volume   = {8},
            number   = {16},
            year     = {2009}
          }
          @incollection{EcVASCONCELLOS2014Entre,
            author     = {Vasconcellos, C. P. V.},
            title      = {Entre representações e estereótipos},
            subtitle   = {o sertão na construção da brasilidade},
            booktitle  = {Culturas dos sertões},
            editor     = {Pereira, A.},
            editortype = {organizer},
            location   = {Salvador},
            publisher  = {EdUFBA},
            year       = {2014}
          }
          @book{EcWOLTON1996Elogio,
            title     = {Elogio do Grande Público},
            subtitle  = {Uma teoria crítica da televisão},
            author    = {Wolton, D.},
            year      = {1996},
            publisher = {Ática},
            location  = {São Paulo}
          }
          @article{EcCÁDIMA2001Proto,
            author   = {Cádima, Francisco Rui},
            title    = {Proto e pós-televisão. Adorno, Bourdieu e os outros—ou na pista da «qualimetria»},
            location = {Lisboa},
            journal  = {Revista de Comunicação e Linguagens},
            year     = {2001},
            number   = {30}
          }
          @book{EcSCOLARIHipermediaciones,
            author    = {Scolari, Carlos},
            title     = {Hipermediaciones},
            subtitle  = {Elementos para una teoría de la comunicación digital interactiva},
            publisher = {Gedisa},
            location  = {Barcelona},
            year      = {2008}
          }
          @book{EcARONCHI2015Gêneros,
            author    = {de Souza, Jose Carlos Aronchi},
            title     = {Generos e Formatos na Televisao Brasileira},
            publisher = {Summus Editorial},
            location  = {São Paulo},
            year      = {2015}
          }
          @article{EcSILVA2017Aspectos,
            title   = {Aspectos do imaginário e da comunicação em Grande Sertão: Veredas},
            number  = {40},
            journal = {Intexto},
            author  = {Silva, Gustavo Castro},
            year    = {2017},
            month   = {8},
            pages   = {96--113}
          }
          @article{EcSANTOS2018Comunicação,
            author  = {Santos, B. S.},
            title   = {A Comunicação sob o olhar de Boaventura de Sousa Santos. [Entrevista concedida a] Eloisa Loose},
            journal = {Ação Midiática --- Estudos em Comunicação, Sociedade e Cultura.},
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
            r#"O objeto deste artigo é a série _Onde nascem os fortes_ (TV Globo, 2018), escrita para exibição em canal aberto de televisão, em ano eleitoral e filmada no cariri paraibano. A partir do título e da ambiência, percebemos uma configuração que remete ao livro _Os sertões_ (1902). Objetiva-se perscrutar como o conceito de sertão é trabalhado na obra, identificar a dialogia com o livro euclidiano e investigar o modo como as desigualdades sociais detectadas pelo escritor no início do século XX permanecem neste século XXI com impressionante atualidade. Ademais, o território sertanejo revela-se como poderoso cronotopo (BAKHTIN, 2003), em forte simetria com a linha abissal da Sociologia das Ausências (SANTOS, 2004). Elege-se o capítulo de estreia como evidenciador de pontos fundamentais da diegese, a partir de metodologia baseada na técnica da minutagem, através da qual analisa-se as estratégias de construção narrativa (MOTTA, 2013), bem como os procedimentos de elaboração do roteiro (MACIEL, 2017). Conclui-se que o episódio inaugural figura como síntese importante para o desenvolvimento da trama, apresentando cenas nas quais diversas percepções destacadas por Euclides da Cunha aparecem e dão pistas de como o roteiro prosseguirá, embora trazendo ressignificações para o espaço sertanejo e os personagens que o habitam."#
        )
    }
}
