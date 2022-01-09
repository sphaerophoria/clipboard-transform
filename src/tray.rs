use crate::config::{Mapping, MappingState};

use ksni::{MenuItem, Tray};
use log::error;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct MonitorTray {
    config_path: PathBuf,
    mappings: Arc<Mutex<Vec<Mapping>>>,
}

impl MonitorTray {
    pub fn new(mappings: Arc<Mutex<Vec<Mapping>>>, config_path: PathBuf) -> MonitorTray {
        MonitorTray {
            config_path,
            mappings,
        }
    }

    pub fn update_mappings(&mut self, mappings: Arc<Mutex<Vec<Mapping>>>) {
        self.mappings = mappings;
    }
}

impl Tray for MonitorTray {
    fn icon_name(&self) -> String {
        "edit-copy".into()
    }

    fn title(&self) -> String {
        "Clipboard transforms".into()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut ret = Vec::new();
        let mappings = self.mappings.lock().expect("Poisoned mappings lock");
        use ksni::menu::*;
        for (idx, mapping) in mappings.iter().enumerate() {
            match mapping {
                Mapping::Simple(mapping) => {
                    let mut menu_item = SubMenu {
                        label: format!("{} ↔ {}", mapping.left, mapping.right).replace("_", "__"),
                        ..Default::default()
                    };

                    let radio_items = vec![
                        RadioItem {
                            label: "Disabled".into(),
                            ..Default::default()
                        },
                        RadioItem {
                            label: "Left to right".into(),
                            ..Default::default()
                        },
                        RadioItem {
                            label: "Right to left".into(),
                            ..Default::default()
                        },
                    ];

                    let callback_idx = idx;
                    let radio_select = Box::new(move |tray: &mut Self, radio_idx| {
                        let mut mappings = tray.mappings.lock().expect("Poisoned mappings lock");
                        let mapping = match &mut mappings[callback_idx] {
                            Mapping::Simple(mapping) => mapping,
                        };
                        mapping.state = match radio_idx {
                            0 => MappingState::Disabled,
                            1 => MappingState::LeftToRight,
                            2 => MappingState::RightToLeft,
                            _ => {
                                error!("Unexpected mapping state");
                                return;
                            }
                        };
                    });

                    let selected = match mapping.state {
                        MappingState::Disabled => 0,
                        MappingState::LeftToRight => 1,
                        MappingState::RightToLeft => 2,
                    };

                    let submenu_item = RadioGroup {
                        select: radio_select,
                        options: radio_items,
                        selected,
                    };

                    menu_item.submenu.push(submenu_item.into());

                    ret.push(menu_item.into())
                }
            }
        }

        if !ret.is_empty() {
            ret.push(MenuItem::Separator);
        }

        let open_item = StandardItem {
            label: "Open config".into(),
            activate: Box::new(|tray: &mut Self| {
                if let Err(e) = open::that(&tray.config_path) {
                    error!("Failed to open config: {}", e);
                }
            }),
            ..Default::default()
        };
        ret.push(open_item.into());

        ret
    }

    fn id(&self) -> String {
        "com.micksayson.clipboard-transforms".into()
    }
}
