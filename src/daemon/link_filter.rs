use adblock::lists::ParseOptions;
use adblock::request::Request;
use adblock::{Engine, FilterSet};
use log::{info, warn};
use lol_html::html_content::Element;
use lol_html::{element, HtmlRewriter, Settings};
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct LinkFilter {
    channel: Sender<Instruct>,
}
enum Instruct {
    Filter(Message),
    Reload(String),
}

type Message = (String, tokio::sync::oneshot::Sender<anyhow::Result<String>>);
impl LinkFilter {
    pub fn new() -> LinkFilter {
        let (sender, receiver) = std::sync::mpsc::channel::<Instruct>();
        std::thread::spawn(move || {
            let mut engine: Option<Engine> = None;
            while let Ok(ins) = receiver.recv() {
                match ins {
                    Instruct::Filter((html, sender)) => {
                        let Some(e) = &engine else {
                            let _ = sender.send(Ok(html));
                            continue;
                        };
                        match filter(e, html.as_bytes()) {
                            Ok(new_html) => {
                                let _ = sender.send(Ok(new_html));
                            }
                            Err(err) => {
                                warn!("Filter error: {}", err);
                                let _ = sender.send(Ok(html));
                            }
                        }
                    }
                    Instruct::Reload(rules) => {
                        let mut filter_set = FilterSet::new(true);
                        filter_set
                            .add_filters(rules.lines().map(|a| a.trim()), ParseOptions::default());
                        engine = Some(Engine::from_filter_set(filter_set, true));
                    }
                }
            }
            info!("LinkFilterDaemon exiting...");
        });
        LinkFilter { channel: sender }
    }

    pub async fn filter(&self, raw: String) -> anyhow::Result<String> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<anyhow::Result<String>>();
        self.channel.send(Instruct::Filter((raw, sender)))?;
        receiver.await?
    }
    pub fn update(&self, rules: String) {
        let _ = self.channel.send(Instruct::Reload(rules));
    }
}
fn filter(engine: &Engine, raw: &[u8]) -> anyhow::Result<String> {
    let mut output = vec![];
    let link = |el: &mut Element| {
        let Some(href) = el.get_attribute("href") else {
            return Ok(());
        };
        let Ok(request) = Request::new(&href, "", "fetch") else {
            return Ok(());
        };
        let result = engine.check_network_request(&request);
        if result.matched {
            let _ = el.set_attribute("href", "#blocked");
            let _ = el.set_attribute("class", "link-blocked");
            let _ = el.set_attribute("title", &format!("Original link {} blocked", href));
        } else if let Some(new_url) = result.rewritten_url {
            let _ = el.set_attribute("href", &new_url);
        }
        Ok(())
    };
    let img = |el: &mut Element| {
        let Some(src) = el.get_attribute("src") else {
            return Ok(());
        };
        let Ok(request) = Request::new(&src, "", "image") else {
            return Ok(());
        };
        let result = engine.check_network_request(&request);
        if result.matched {
            let _ = el.set_attribute("src", "#blocked");
            let _ = el.set_attribute("class", "image-blocked");
            let _ = el.set_attribute("alt", &format!("Original src {} blocked", src));
        } else if let Some(new_url) = result.rewritten_url {
            let _ = el.set_attribute("src", &new_url);
        }
        Ok(())
    };
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![element!("a[href]", link), element!("img[src]", img)],
            ..Settings::default()
        },
        |c: &[u8]| {
            output.extend_from_slice(c);
        },
    );

    rewriter.write(raw)?;
    rewriter.end()?;
    Ok(String::from_utf8(output)?)
}
