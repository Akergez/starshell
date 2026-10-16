const XKB_REGISTRY: &str = "/usr/share/X11/xkb/rules/evdev.xml";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyboardLayouts {
    pub names: Vec<String>,
    pub labels: Vec<String>,
    pub current: usize,
}

impl KeyboardLayouts {
    pub fn new(names: Vec<String>, current: usize) -> Self {
        let registry = std::fs::read_to_string(XKB_REGISTRY).unwrap_or_default();
        let labels = names
            .iter()
            .map(|name| indicator(name, &registry))
            .collect();
        Self {
            names,
            labels,
            current,
        }
    }

    pub fn switched(&self, current: usize) -> Self {
        Self {
            current,
            ..self.clone()
        }
    }

    pub fn current_label(&self) -> Option<&str> {
        self.labels.get(self.current).map(String::as_str)
    }

    pub fn current_name(&self) -> Option<&str> {
        self.names.get(self.current).map(String::as_str)
    }
}

fn tag<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.strip_prefix('<')?
        .strip_prefix(name)?
        .strip_prefix('>')?
        .strip_suffix('>')?
        .strip_suffix(name)?
        .strip_suffix("</")
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn indicator(description: &str, registry: &str) -> String {
    let mut in_layouts = false;
    let mut in_variants = false;
    let mut layout_short: Option<&str> = None;
    let mut item_short: Option<&str> = None;
    for line in registry.lines().map(str::trim) {
        match line {
            "<layoutList>" => in_layouts = true,
            "</layoutList>" => in_layouts = false,
            "<layout>" => {
                layout_short = None;
                in_variants = false;
            }
            "<variantList>" => in_variants = true,
            "<configItem>" => item_short = None,
            _ if in_layouts => {
                if let Some(short) = tag(line, "shortDescription") {
                    item_short = Some(short);
                    if !in_variants {
                        layout_short = Some(short);
                    }
                } else if let Some(text) = tag(line, "description")
                    && unescape(text) == description
                    && let Some(short) = item_short.or(layout_short)
                {
                    return short.to_uppercase();
                }
            }
            _ => {}
        }
    }
    description
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &str = "<xkbConfigRegistry>
  <modelList>
    <model>
      <configItem>
        <name>pc105</name>
        <description>English (US)</description>
      </configItem>
    </model>
  </modelList>
  <layoutList>
    <layout>
      <configItem>
        <name>us</name>
        <shortDescription>en</shortDescription>
        <description>English (US)</description>
      </configItem>
      <variantList>
        <variant>
          <configItem>
            <name>dvorak</name>
            <description>English (Dvorak)</description>
          </configItem>
        </variant>
      </variantList>
    </layout>
    <layout>
      <configItem>
        <name>ru</name>
        <shortDescription>ru</shortDescription>
        <description>Russian</description>
      </configItem>
      <variantList>
        <variant>
          <configItem>
            <name>tt</name>
            <shortDescription>tt</shortDescription>
            <description>Tatar</description>
          </configItem>
        </variant>
      </variantList>
    </layout>
    <layout>
      <configItem>
        <name>ba</name>
        <shortDescription>bs</shortDescription>
        <description>Bosnian &amp; more</description>
      </configItem>
    </layout>
  </layoutList>
</xkbConfigRegistry>";

    #[test]
    fn layouts_show_their_xkb_indicator() {
        assert_eq!(indicator("English (US)", REGISTRY), "EN");
        assert_eq!(indicator("Russian", REGISTRY), "RU");
        assert_eq!(indicator("Bosnian & more", REGISTRY), "BS");
    }

    #[test]
    fn variants_fall_back_to_their_layout_indicator() {
        assert_eq!(indicator("English (Dvorak)", REGISTRY), "EN");
        assert_eq!(indicator("Tatar", REGISTRY), "TT");
    }

    #[test]
    fn unknown_layouts_use_their_first_letters() {
        assert_eq!(indicator("Klingon", REGISTRY), "KL");
    }

    #[test]
    fn switching_keeps_the_labels() {
        let layouts = KeyboardLayouts {
            names: vec!["English (US)".into(), "Russian".into()],
            labels: vec!["EN".into(), "RU".into()],
            current: 0,
        };
        let switched = layouts.switched(1);
        assert_eq!(switched.current_label(), Some("RU"));
        assert_eq!(switched.current_name(), Some("Russian"));
    }
}
