# ethornell-bsi-dumper

Ethornell (BGI, Buriko General Interpreter) BSI file dumper.

Ethornell is a visual novel game engine, and BSI is a file format for the engine that contains strings for translated scripts.

This repository contains the code I used to dump the translated scripts for Amairo Chocolate (あまいろショコラータ) 1 and 2 to create ebooks for personal use.

Please note that I only spent 18 hours researching Ethornell and writing this program, and it has worked perfectly for Amairo Chocolate 1 and 2, but it is not a program that is generic and complete for all Ethornell games. So I've written this README in detail, explaining how it works so you should be able to easily modify it for the game you're working on.

## BSI Data Structure

It's simple, here's the C pseudo-code for it

```cpp

struct BSI {
    uint32_t magic; // or version
    Block blocks[];
};

struct Block {
    char8_t name[];
    uint32_t id; // unknown usage
    (ItemText | ItemMapping | ItemScript) items[];
};

struct ItemText {
    char8_t text[];
};

struct ItemMapping {
    char8_t original[];
    char8_t translated[];
};

struct ItemScript {
    // The index of this script in `Block.items`
    // 6-digit decimal string, starts from "000000"
    char8_t index[7];
    char8_t text[];
};
```

For Amairo Chocolate 1 and 2 (hereinafter referred to as AC1 and AC2)

### `BSI.magic`

`0xA` for AC1, `0xE` for AC2.

### `BSI.blocks[]`

For AC1

| Field `.name`       | Type of `.items` | Explanation                                            |
|---------------------|------------------|--------------------------------------------------------|
| `"[name]"`          | `ItemText[]`     | Original (untranslated) character names                |
| `"AC_00common"`     | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC_01mikuri"`     | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC_02chieri"`     | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC_03harem"`      | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC_06general"`    | `ItemScript[]`   | Translated scripts for game stories                    |
| `"[name2]"`         | `ItemMapping[]`  | Character name translation mappings                    |
| `"[selection]"`     | `ItemMapping[]`  | Branching option translation mappings for game stories |
| `"[system]"`        | `ItemMapping[]`  | System text translation mappings                       |
| `"_LanguageSelect"` | `ItemScript[]`   | Translated scripts for the language selection scene.   |

For AC2

| Field `.name`       | Type of `.items` | Explanation                                            |
|---------------------|------------------|--------------------------------------------------------|
| `"AC2_00common"`    | `ItemScript[]`   | Translated scripts for game stories                    |
| `"[name]"`          | `ItemText[]`     | Original (untranslated) character names                |
| `"AC2_01sweet"`     | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_10ichika"`    | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_19ichika"`    | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_20nana"`      | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_29nana"`      | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_30kaguya"`    | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_39kaguya"`    | `ItemScript[]`   | Translated scripts for game stories                    |
| `"AC2_40harem"`     | `ItemScript[]`   | Translated scripts for game stories                    |
| `"[name2]"`         | `ItemMapping[]`  | Character name translation mappings                    |
| `"[selection]"`     | `ItemMapping[]`  | Branching option translation mappings for game stories |
| `"[system]"`        | `ItemMapping[]`  | System text translation mappings                       |
| `"_LanguageSelect"` | `ItemScript[]`   | Translated scripts for the language selection scene    |

## Dump Process

Basically, we just need to parse the BSI file according to the data structure, and the names of the script translation blocks of the game's story start with `"AC_"` and `"AC2_"` for AC1 and AC2, which for me are the most interesting parts.

Note that strings in a BSI file are UTF-8 encoded, so a single `'\0'` character cannot simply be treated as the end of a string.

But, after dumping, we'll see that these `ItemScript`s don't contain information about who said each script, and even the entire BSI file doesn't contain this information. This information is stored in original script files (packaged in one or more `.arc` files).

You probably extracted the BSI file from an ARC file as well (if you don't know, try using [GARbro](https://github.com/morkt/GARbro)), so look for original script files in other ARC files in the same way, which have the same name as game story `ItemScript` blocks (only the case is different).

For AC1, `ac_00common`, `ac_01mikuri` and `ac_02chieri` are in `data01100.arc`, `ac_03harem` and `ac_06general` are in `data01111.arc`.

For AC2, `ac2_00common`, `ac2_01sweet`, `ac2_10ichika`, `ac2_20nana` and `ac2_30kaguya` are in `data01120.arc`, `ac2_19ichika`, `ac2_29nana`, `ac2_39kaguya` and `ac2_40harem` are in `data01121.arc`.

These binary files we just extracted are scripts compiled by Buriko (starting with bytes `"BurikoCompiledScriptVer1.00"` as magic), so we need a tool to decompile them. I used [VNTranslationTools.VNTextPatch](https://github.com/arcusmaximus/VNTranslationTools). After successful decompilation, we will get their actual JSON files. We will use these JSON files as "references" to find out who said each script.

There's one more thing we need to do before we start looking. These JSON files contain branching option scripts that aren't presented in game story `ItemScript` blocks, so we need to remove all scripts from these JSON files that match the scripts in the `"[selection]"` block first, so that the number of references matches the number of translations.

Finally, we can use the index of the translated script to access who said it in the referenced JSON file, then use it as a key to look up its translated character name in the `"[name2]"` block.

Tip: If you can't find certain game story `ItemScript` block names in all ARC files, maybe it's because they contain R-18 content and you don't have the R-18 DLC installed. (I wasted a couple of hours before I realized this :/)

## Credits

- [TesterTesterov/bsiTool](https://github.com/TesterTesterov/bsiTool)
