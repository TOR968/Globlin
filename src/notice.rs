use crate::model::{self, Package};

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Announce {
        title: String,
        body: String,
        stamps: Vec<String>,
    },
    Remember {
        stamps: Vec<String>,
    },
    Nothing,
}

pub fn decide(packages: &[Package], last_notified: &[String]) -> Decision {
    let stamps = model::stamps(packages);
    if stamps == last_notified {
        return Decision::Nothing;
    }
    if stamps.is_empty() {
        return Decision::Remember { stamps };
    }
    Decision::Announce {
        title: title(packages),
        body: body(packages),
        stamps,
    }
}

fn title(packages: &[Package]) -> String {
    match model::outdated(packages).len() {
        1 => "1 npm global update".to_string(),
        count => format!("{count} npm global updates"),
    }
}

fn body(packages: &[Package]) -> String {
    model::outdated(packages)
        .iter()
        .map(|package| {
            format!(
                "{}{}  {} → {}",
                package.name,
                package.source.suffix(),
                package.current,
                package
                    .latest()
                    .map(ToString::to_string)
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn self_failure(version: &str, last: Option<&str>) -> bool {
    last != Some(version)
}

#[cfg(test)]
mod tests;
