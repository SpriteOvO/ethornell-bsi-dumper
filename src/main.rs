use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    path::PathBuf,
};

use clap::Parser;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Parser)]
struct CliArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    reference_dir: Vec<PathBuf>,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_delimiter = ',')]
    formats: Vec<String>,
}

#[derive(Clone, Debug)]
enum Item {
    Index(usize),
    Script(String),
}

impl Item {
    fn into_content(self) -> Option<String> {
        match self {
            Self::Index(_) => None,
            Self::Script(content) => Some(content),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data")]
enum Data {
    List(Vec<String>),
    Map(HashMap<String, String>),
    Scripts(Vec<Script>),
}

impl Data {
    fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }

    fn as_map(&self) -> Option<&HashMap<String, String>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    fn as_scripts_mut(&mut self) -> Option<&mut Vec<Script>> {
        match self {
            Self::Scripts(scripts) => Some(scripts),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum Speaker {
    #[serde(serialize_with = "empty_str")]
    NoName,
    Named(String),
}

fn empty_str<S: Serializer>(s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("")
}

#[derive(Clone, Debug, Serialize)]
struct Script {
    speaker: Option<Speaker>,
    #[serde(skip)]
    index: Option<usize>,
    text: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Reference(Vec<ReferenceScript>);

#[derive(Clone, Debug, Deserialize)]
struct ReferenceScript {
    name: Option<String>,
    message: String,
}

fn merge_block(unmerged: BlockUnmerged) -> Block {
    let data = if unmerged
        .items
        .iter()
        .all(|item| matches!(item, Item::Script(_)))
    {
        if unmerged.name == "[name2]" {
            Data::Map(HashMap::from_iter(unmerged.items.chunks(2).map(|chunk| {
                (
                    chunk[0].clone().into_content().unwrap(),
                    chunk[1].clone().into_content().unwrap(),
                )
            })))
        } else {
            Data::List(
                unmerged
                    .items
                    .into_iter()
                    .map(|item| item.into_content().unwrap())
                    .collect(),
            )
        }
    } else {
        Data::Scripts(
            unmerged
                .items
                .chunks(2)
                .map(|chunk| {
                    assert_eq!(chunk.len(), 2);
                    match (&chunk[0], &chunk[1]) {
                        (Item::Index(index), Item::Script(script)) => Script {
                            speaker: None,
                            index: Some(*index),
                            text: script.clone(),
                        },
                        _ => panic!("{unmerged:?}"),
                    }
                })
                .collect(),
        )
    };
    Block {
        name: unmerged.name,
        id: unmerged.id,
        data,
    }
}

fn merge_speakers(reference_dir: &[PathBuf], mut blocks: Vec<Block>) -> Vec<Block> {
    let Some(selections) = blocks
        .iter()
        .find(|block| block.name == "[selection]")
        .map(|block| block.data.as_list().unwrap().to_vec())
    else {
        eprintln!("!!! selections not found");
        return blocks;
    };
    blocks
        .iter_mut()
        .filter(|block| is_story_block(&block.name))
        .for_each(|block| {
            if let Some(scripts) = block.data.as_scripts_mut() {
                // TODO: FIXME, don't eprintln in tries
                let reference = reference_dir.iter().find_map(|reference_dir| {
                    let reference_file = reference_dir
                        .join(block.name.to_ascii_lowercase())
                        .with_extension("json");
                    fs::read(&reference_file)
                        .inspect_err(|err| {
                            eprintln!("!!! failed to read reference file '{reference_file:?}': {err}")
                        })
                        .ok()
                        .and_then(|bytes| {
                            serde_json::from_slice::<Reference>(&bytes)
                                .inspect_err(|err| {
                                    eprintln!(
                                        "!!! failed to parse reference file '{reference_file:?}': {err}"
                                    )
                                })
                                .ok()
                        })
                        .map(|mut reference| {
                            reference
                                .0
                                .retain(|script| !selections.contains(&script.message));
                            reference
                        })
                        .inspect(|_| println!("used reference file '{reference_file:?}'"))
                });

                if let Some(reference) = &reference {
                    assert_eq!(reference.0.len(), scripts.len());
                }

                scripts.iter_mut().enumerate().for_each(|(i, script)| {
                    script.speaker = reference.as_ref().map(|reference| {
                        // assert_eq!(
                        //     reference.0[i].message, script.text,
                        //     "block: {}",
                        //     block.name
                        // );

                        reference.0[i]
                            .name
                            .as_ref()
                            .map_or(Speaker::NoName, |name| Speaker::Named(name.clone()))
                    });
                });
            }
        });
    blocks
}

fn translate_speakers(mut blocks: Vec<Block>) -> Vec<Block> {
    let Some(name_mapping) = blocks
        .iter()
        .find(|block| block.name == "[name2]")
        .map(|block| block.data.as_map().unwrap())
        .cloned()
    else {
        eprintln!("!!! name mapping not found");
        return blocks;
    };

    let mut untranslated_speakers = HashSet::new();

    blocks
        .iter_mut()
        .filter_map(|block| block.data.as_scripts_mut())
        .for_each(|scripts| {
            scripts.iter_mut().for_each(|script| {
                if let Some(Speaker::Named(speaker)) = &mut script.speaker {
                    if let Some(translated) = name_mapping.get(speaker) {
                        *speaker = translated.clone();
                    } else {
                        untranslated_speakers.insert(speaker);
                    }
                }
            })
        });

    untranslated_speakers
        .iter()
        .for_each(|speaker| eprintln!("!!! unable to translate character '{speaker}'"));
    blocks
}

#[derive(Debug)]
struct BlockUnmerged {
    name: String,
    id: u32,
    items: Vec<Item>,
}

#[derive(Clone, Debug, Serialize)]
struct Block {
    name: String,
    #[serde(skip)]
    id: u32,
    #[serde(flatten)]
    data: Data,
}

fn to_markdown(blocks: impl IntoIterator<Item = Block>) -> String {
    let mut ret = String::new();
    for block in blocks {
        if let Data::Scripts(scripts) = block.data {
            ret.push_str("# ");
            ret.push_str(&block.name);
            ret.push_str("\n\n");

            for script in scripts {
                if let Some(Speaker::Named(name)) = &script.speaker {
                    ret.push_str(name);
                    ret.push('\n');
                }
                ret.push_str(&script.text);
                ret.push_str("\n\n");
            }
        }
    }
    ret
}

fn is_story_block(name: &str) -> bool {
    let lowercase = name.to_ascii_lowercase();
    lowercase.starts_with("ac_") || lowercase.starts_with("ac2_")
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();

    let mut input = fs::read(args.input)?;
    let magic = u32::from_le_bytes(input[..4].try_into().unwrap());
    if magic != 0xA && magic != 0xE {
        panic!("unknown magic {magic}");
    }
    input = input[4..].into();

    let mut unmerged_blocks = vec![];
    let mut next_expected_index = 0;
    while !input.is_empty() {
        let chunk = String::from_utf8(input.clone()).unwrap_or_else(|err| {
            String::from_utf8(input[0..err.utf8_error().valid_up_to()].into()).unwrap()
        });
        let item = chunk.split('\0').next().unwrap();

        assert_eq!(input[item.len()], 0);
        input = input[item.len() + 1..].into();

        // Block starts
        if item.starts_with('[') || item.starts_with('_') || is_story_block(item) {
            unmerged_blocks.push(BlockUnmerged {
                name: item.into(),
                id: u32::from_le_bytes(input[..4].try_into().unwrap()),
                items: vec![],
            });
            next_expected_index = 0;

            input = input[4..].into();
        }
        // Item
        else {
            let item = if item.chars().count() == 6 && item.chars().all(|ch| ch.is_ascii_digit()) {
                let index = item.parse()?;
                assert_eq!(index, next_expected_index);
                next_expected_index += 1;
                Item::Index(index)
            } else {
                Item::Script(item.into())
            };
            unmerged_blocks.last_mut().unwrap().items.push(item);
        }
    }

    let blocks = translate_speakers(merge_speakers(
        &args.reference_dir,
        unmerged_blocks
            .into_iter()
            .map(merge_block)
            .collect::<Vec<_>>(),
    ));

    blocks
        .iter()
        .filter(|block| matches!(block.data, Data::Scripts(_)))
        .for_each(|block| println!("reference dependency: {}", block.name));

    for format in args.formats {
        let content = match format.as_str() {
            "json" => serde_json::to_string_pretty(&blocks).unwrap(),
            "md" => to_markdown(blocks.clone()),
            "txt" => to_markdown(blocks.clone()),
            format => panic!("unsupported format '{format}'"),
        };
        let path = args.output.with_extension(format);
        println!("writing to '{path:?}'");
        fs::write(path, content)?;
    }

    Ok(())
}
