extern crate bib_parser;
extern crate handlebars;
extern crate pandoc;

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use bib_parser::Entry;
use handlebars::Handlebars;
use pandoc::OutputFormat::Html5;
use pandoc::InputFormat::Markdown;
use pandoc::OutputKind::Pipe as OutputPipe;
use pandoc::InputKind::Pipe as InputPipe;
use pandoc::PandocOutput::ToBuffer;
use pandoc::PandocOption::MathJax;

fn read_bib(bib_path: &str) -> Vec<(String, Option<Entry>)> {
    let mut file = File::open(bib_path).expect("Could not open bibliography");
    let mut bs = Vec::new();
    file.read_to_end(&mut bs).unwrap();
    let parsed = bib_parser::parse_bib(&bs);
    match parsed {
        Err(e) => panic!("{:?}", e),
        Ok(x) => x,
    }
}

fn make_index(index: Vec<(String, String, String, String)>) -> String {
    let mut content = String::new();
    let intro = format_args!(r#"<h1>Annotated bibliography</h1>
<p>This is an annotated bibliography of various papers I find interesting. It is automatically generated from a BibTeX file and an archive of Markdown files.</p>
<ul class=\"nonetype\">
"#);
    fmt::write(&mut content, intro).unwrap();

    for (link, author, year, title) in index {
        fmt::write(
            &mut content,
            format_args!(
                "<li>{} ({}) <a href=\"{}\">{}</a>\n",
                author,
                year,
                link,
                title,
            )
        ).unwrap();
    }
    fmt::write(&mut content, format_args!("</ul>")).unwrap();
    content
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() == 5 {
        // read .bib file
        let entries = read_bib(&args[1]);

        // register handlebars template
        let templ_path = PathBuf::from(&args[2]);
        let mut hbs = Handlebars::new();
        let mut templ_file = File::open(templ_path)
            .expect("Could not open template file");
        let mut templ_contents = String::new();
        templ_file.read_to_string(&mut templ_contents)
            .expect("Could not read template file");
        hbs.register_template_string("t", templ_contents)
            .expect("Could not register template");

        // this vector will be used to create an index for the notes
        let mut index = Vec::new();

        // output all the individual files
        let markdown_path = PathBuf::from(&args[3]);
        let output_path = PathBuf::from(&args[4]);
        for (key, entry) in entries {
            let entry = match entry {
                Some(entry) => entry,
                None => continue,
            };

            // read the markdown source, or continue if it doesn't exist
            let md_name = format!("{}.md", &key);
            let md_path = markdown_path.join(md_name);
            let mut md_file = match File::open(md_path) {
                Err(_) => continue,
                Ok(f) => f,
            };
            let mut md_contents = String::new();
            md_file.read_to_string(&mut md_contents)
                .expect("Could not read markdown file");
            
            // set up and run pandoc
            let mut pandoc = pandoc::new();
            pandoc.set_output_format(Html5)
                  .set_output(OutputPipe)
                  .set_input_format(Markdown)
                  .set_input(InputPipe(md_contents))
                  .add_option(MathJax(None));
            let pandoc_output = pandoc.execute().expect("Could not run pandoc");

            // extract the output
            let body = match pandoc_output {
                ToBuffer(s) => s,
                _ => unreachable!(),
            };

            // add the header
            let rendered = format!(
                "<header><h1>{}</h1><cite>{} ({}) <em>{}</em></cite></header>\n{}",
                entry.title(),
                entry.author().to_string(),
                entry.year(),
                entry.title(),
                body
            );

            // run handlebars
            let mut data = BTreeMap::new();
            data.insert("title", entry.title());
            data.insert("content", &rendered);
            let rendered_again = hbs.render("t", &data)
                .expect("Handlebars failed to run");

            // write output
            let html_name = format!("{}.html", &key);
            let html_path = output_path.join(html_name.clone());
            let mut html_file = File::create(html_path)
                .expect("Could not open output file");
            writeln!(html_file, "{}", rendered_again)
                .expect("Could not write to output file");
            
            index.push((
                html_name,
                entry.author().to_string(),
                entry.year().to_string(),
                entry.title().to_owned()
            ));
        }

        // now build the index
        let index_contents = make_index(index);
        let mut data = BTreeMap::new();
        data.insert("title", "Annotated bibliography");
        data.insert("content", &index_contents);
        let rendered_index = hbs.render("t", &data)
            .expect("Handlebars failed to run");

        // write output
        let index_path = output_path.join("index.html");
        let mut index_file = File::create(index_path)
            .expect("Could not open index file");
        writeln!(index_file, "{}", rendered_index)
            .expect("Could not write to index file");
    } else {
        writeln!(
            &mut std::io::stderr(),
            "syntax: biblionotes <bibliography> <template> <markdown_dir> <output_dir>"
        ).unwrap();
    }
}