use serde_derive::Deserialize;
use toml;
use whoami;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config{
    pub window:  WindowProps,
    pub margins: MarginProps,
    pub font:    FontProps,
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub enum Placement{
    TOP,
    BOTTOM,
    LEFT,
    RIGHT,
    CENTER_VERTICAL,
    CENTER_HORIZONTAL,
}

#[derive(Debug, Deserialize)]
pub struct WindowProps{
    pub width: u32,
    pub height: u32,
    pub background_color: u32,
    win_position_str: String,
    pub win_position: Option<(Placement, Placement)>,
}
impl WindowProps{
    pub fn calc_win_position(&mut self) {

        let mut full_placement = (Placement::CENTER_VERTICAL, Placement::CENTER_HORIZONTAL);

        // Get string to remove whitespaces
        let mut position_stripped_spaces: String = self.win_position_str.clone();
        // Remove whitespaces
        position_stripped_spaces.retain(|c| !c.is_whitespace());

        // Set the properties
        position_stripped_spaces.split(",").for_each(|position| {
            match position{
                "CENTER_HORIZONTAL" => { full_placement.0 = Placement::CENTER_HORIZONTAL; /*config.window.win_anchor_int |= 0;8*/ },
                "CENTER_VERTICAL"   => full_placement.1 = Placement::CENTER_VERTICAL,
                "LEFT"              => full_placement.0 = Placement::LEFT,
                "RIGHT"             => full_placement.0 = Placement::RIGHT,
                "TOP"               => full_placement.1 = Placement::TOP,
                "BOTTOM"            => full_placement.1 = Placement::BOTTOM,
                _ => ()
            }
        });

        self.win_position = Some(full_placement.clone());
    }
}

#[derive(Debug, Deserialize)]
pub struct MarginProps{
    pub vertical_percentage:   u8,
    pub horizontal_percentage: u8,
}

#[derive(Debug, Deserialize)]
pub struct FontProps{
    pub name:  String,
    pub size:  u8,
    pub color: u32,
}



static DEFAULT_CONFIG: &str = r#"
        [window]
        width  = 100
        height = 100
        background_color = 0x262626

        # Possible values are {CENTER_VERTICAL, CENTER_HORIZONTAL, TOP, BOTTOM, LEFT, RIGHT}
        win_position_str = 'CENTER_VERTICAL, CENTER_HORIZONTAL'

        [margins]
        vertical_percentage   = 10
        horizontal_percentage = 10

        [font]
        name  = 'Roboto Condensed'
        size  = 15
        color = 0x808080
    "#;


pub fn init_toml_config(config_name: Option<String>) -> Config{

    let mut config: Config;

    if let Some(conf_name) = config_name{
        // If the config name is specified, open it in the .conf directory and parse it
        let path: String = format!("{}{}{}{}{}", "/home/", whoami::username(), "/.config/gwstuff/", conf_name, ".toml");
        config = toml::from_str(&fs::read_to_string(path).expect("Invalid file path")).expect("Invalid TOML config file");
    }
    else{
        // If no filename is given, load default config
        config = toml::from_str(DEFAULT_CONFIG).expect("Invalid DEFAULT_CONFIG");
    }

    config.window.calc_win_position();

    config
}
