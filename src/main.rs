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
use petgraph::{Graph};
use serde_json::{Map, Value};
use std::{borrow:: Cow, clone::Clone, io::Write, fs::File, path::Path};

#[derive(Deserialize, Debug, Clone)]
struct Package {
    name: String,
    version: String,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Map<String, Value>,
    dependencies: Map<String, Value>,
}

type DepGraph<'a> = Graph<Package, &'a str>;

impl Package {
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

type Nd = usize;
type Ed = (usize, usize);
#[derive(Debug)]
struct Edges(Vec<Ed>);

impl<'a> dot::Labeller<'a, Nd, Ed> for Edges {
    fn graph_id(&'a self) -> dot::Id<'a> { dot::Id::new("package").unwrap() }
    
    fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
        dot::Id::new(format!("N{}", *n)).unwrap()
    }
}

impl<'a> dot::GraphWalk<'a, Nd, Ed> for Edges {
    fn nodes(&self) -> dot::Nodes<'a,Nd> {
        // (assumes that |N| \approxeq |E|)
        let &Edges(ref v) = self;
        let mut nodes = Vec::with_capacity(v.len());
        for &(s,t) in v {
            nodes.push(s); nodes.push(t);
        }
        nodes.sort();
        nodes.dedup();
        Cow::Owned(nodes)
    }
    
    fn edges(&'a self) -> dot::Edges<'a,Ed> {
        let &Edges(ref edges) = self;
        Cow::Borrowed(&edges[..])
    }
    
    fn source(&self, e: &Ed) -> Nd { e.0 }
    
    fn target(&self, e: &Ed) -> Nd { e.1 }
}



fn render_to<W: Write>(output: &mut W, graph: DepGraph) {
    let (nodes, edges) = graph.into_nodes_edges();
    
    let mut mapped_edges = Edges(Vec::new());
    for edg in edges {
        let ed: Ed = (edg.source().index(), edg.target().index());
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
