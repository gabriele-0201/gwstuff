use serde_derive::Deserialize;
use toml;
use whoami;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub window:  WindowProps,
    pub margins: MarginProps,
    pub font:    FontProps,
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub enum Placement{
    Top,
    Bottom,
    Left,
    Right,
    CenterVertical,
    CenterHorizontal,
}

impl Placement{
    pub fn to_raw(self) -> u32{
        match self{
            Placement::Top => 1,
            Placement::Bottom => 2,
            Placement::Left => 4,
            Placement::Right => 8,
            Placement::CenterVertical => 3,
            Placement::CenterHorizontal => 12,
        }
    }
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub enum TextAlignment {
    Center, 
    Left, 
    Right,
}

#[derive(Debug, Deserialize)]
pub struct WindowProps{
    pub width: u32,
    pub height: u32,
    pub background_color: u32,
    win_position_str: String,
    pub win_position: (Placement, Placement),
}
impl WindowProps{
    pub fn calc_win_position(&mut self) {

        let mut full_placement = (Placement::CenterVertical, Placement::CenterHorizontal);


        // Get string to remove whitespaces
        let mut position_stripped_spaces: String = self.win_position_str.clone();
        // Remove whitespaces
        position_stripped_spaces.retain(|c| !c.is_whitespace());

        // Set the properties
        position_stripped_spaces.split(",").for_each(|position| {
            match position{
                "CenterHorizontal" => full_placement.0 = Placement::CenterHorizontal,
                "CenterVertical"   => full_placement.1 = Placement::CenterVertical,
                "Left"              => full_placement.0 = Placement::Left,
                "Right"             => full_placement.0 = Placement::Right,
                "Top"               => full_placement.1 = Placement::Top,
                "Bottom"            => full_placement.1 = Placement::Bottom,
                _ => ()
            }
        });

        self.win_position = full_placement.clone();
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
    pub size:  f32,
    pub color: u32,
    pub intra_line: f32,
    pub text_alignment :TextAlignment
}



static DEFAULT_CONFIG: &str = r#"
        [window]
        width  = 600
        height = 600
        background_color = 0x262626

        # Possible values are {CenterVertical, CenterHorizontal, Top, Bottom, Left, Right}
        win_position_str = 'Top, Left'

        [margins]
        vertical_percentage   = 5
        horizontal_percentage = 5

        [font]
        name  = 'Roboto Condensed'
        size  = 15
        color = 0x808080
        #intra_line = 1.0
    "#;


pub fn init_toml_config(config_name: Option<String>) -> Config {

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
