#[macro_use]
extern crate clap;
extern crate dot;
extern crate failure;
//#[macro_use]
//extern crate failure_derive;
extern crate petgraph;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod constants;

use clap::{App, Arg};
use constants::FILE_ARG;
use petgraph::{Graph, visit::EdgeRef};
use serde_json::{Map, Value};
use std::{borrow:: Cow, clone::Clone, io::Write, fs::File, path::Path};

#[derive(Deserialize, Debug, Clone, PartialEq)]
struct Package {
    name: String,
    version: String,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Map<String, Value>,
    dependencies: Map<String, Value>,
}

type DepGraph<'a> = Graph<Package, &'a str>;

impl Package {
    fn new() -> Package {
        Package {
            name: String::new(),
            version: String::new(),
            dev_dependencies: Map::with_capacity(0),
            dependencies: Map::with_capacity(0)
        }
    }
    
    fn new_with_data(name: &str, version: &str) -> Package {
        Package {
            name: name.to_string(),
            version: version.to_string(),
            dev_dependencies: Map::with_capacity(0),
            dependencies: Map::with_capacity(0)
        }
    }
    
    // figure out how to do this without consuming self?
    fn graph_deps<'a>(self, graph: &mut DepGraph) -> () {
        let root = graph.add_node(self.clone());
        for entry in self.dependencies.iter() {
            let (name, version) = entry;
            
            // if unwrap fails then package is invalid
            let package = Package::new_with_data(&name, version.as_str().unwrap());
            let pkg = graph.add_node(package);
            graph.add_edge(root, pkg, "dependency");
        }
    }
}

type Nd<'a> = (usize, &'a str, &'a str);
type Ed<'a> = (Nd<'a>, Nd<'a>, &'a str);
#[derive(Debug)]
struct Edges<'a>(Vec<Ed<'a>>);

impl<'a> dot::Labeller<'a, Nd<'a>, Ed<'a>> for Edges<'a> {
    // TODO: add a way to set this with an arg
    fn graph_id(&'a self) -> dot::Id<'a> { dot::Id::new("package").unwrap() }
    
    fn node_id(&'a self, n: &Nd<'a>) -> dot::Id<'a> {
        dot::Id::new(format!("N{}", n.0)).unwrap()
    }
    
    fn node_label<'b>(&'b self, n: &Nd<'b>) -> dot::LabelText<'b> {
        dot::LabelText::LabelStr(Cow::Owned(format!("{}: {}", n.1.to_string(), n.2.to_string())))
    }
    
    fn edge_label<'b>(&'b self, e: &Ed<'b>) -> dot::LabelText<'b> {
        dot::LabelText::LabelStr(Cow::Owned(e.2.to_string()))
    }
}

// NOTE: not sure if there's a way to walk the actual graph because it's external
impl<'a> dot::GraphWalk<'a, Nd<'a>, Ed<'a>> for Edges<'a> {
    fn nodes(&self) -> dot::Nodes<'a,Nd> {
        // (assumes that |N| \approxeq |E|)
        let &Edges(ref v) = self;
        let mut nodes = Vec::with_capacity(v.len());
        for &(s,t, _) in v {
            nodes.push(s); nodes.push(t);
        }
        nodes.sort();
        nodes.dedup();
        Cow::Owned(nodes)
    }

    fn edges(&'a self) -> dot::Edges<'a, Ed> {
        let &Edges(ref edges) = self;
        Cow::Borrowed(&edges[..])
    }

    fn source(&self, e: &Ed<'a>) -> Nd<'a> { e.0 }

    fn target(&self, e: &Ed<'a>) -> Nd<'a> { e.1 }
}

fn render_to<W: Write>(output: &mut W, graph: DepGraph) {
//    let (nodes, edges) = graph.into_nodes_edges();
    let mut mapped_edges = Edges(Vec::new());
    for edg in graph.edge_references() {
        // NOTE: This is probably dumb because it should always be the same. Consider holding this
        // higher up and just doing a check.
        let src_pkg = &graph[edg.source()];
        let edg_src: Nd = (edg.source().index(), src_pkg.name.as_str(), src_pkg.version.as_str());
    
        let target_pkg = &graph[edg.target()];
        let edg_target: Nd = (edg.target().index(), target_pkg.name.as_str(), target_pkg.version.as_str());
        let edg_weight = *edg.weight();
        let ed: Ed = (edg_src, edg_target, edg_weight);

        mapped_edges.0.push(ed);
    }

    dot::render(&mapped_edges, output).unwrap()
}


fn main() {
    let matches = App::new("Graph Deps")
        .version(crate_version!())
        .author("Thomas Kastanek")
        .about("Graph your deps")
        .arg(Arg::with_name(FILE_ARG)
            .help("file path")
            .multiple(true)
            .value_name("FILE"))
        .get_matches();
    
    
    if matches.is_present(FILE_ARG) {
        let mut deps = DepGraph::new();
        for arg in matches.values_of(FILE_ARG).unwrap() {
            let file_path = Path::new(arg);
            
            // TODO: check for JSON
            match File::open(file_path) {
                Err(_) => println!("BAD PATH"),
                Ok(mut file) => {
                    let p: Package = serde_json::from_reader(file).unwrap();
                    p.graph_deps(&mut deps);
                }
            }
        }
        let out_path = Path::new("graph.dot");
        let mut out_file = File::create(out_path).unwrap();
        

        render_to(&mut out_file, deps);
//        println!("{:?}", &dot_file);
    }
}
