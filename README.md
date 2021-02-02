# Aspirational Features

1. Quad Store for storing triples
   1. Awesome if it could do RDF*
2. Categories of Named Graphs:
   1. Ontology
   2. Ontology inferences, like parent relationships closures to make certain queries faster
   3. Data, which uses the Ontology
   4. Inferences on the Data
3. These categories of graph data should be essentially automatically maintained.
4. A REST API to go along with the functionality here
5. A Command Line tool to interact locally
6. Align names of named graphs with loaded @base/ontology URIs
7. Smart sparql queries on data, grabbing inferred data

## Named Graph Categories

### Data

* Data model URI and named graph URI should be the same. We will call call the collection of triples in a Data Named Graph a Model.
  The URI for a Model we will denote `<M>`.
* A Model can have inferences, or not. If a Model has inferences, then those will exist in another
  A Model with an accompanying Inferences will be denoted `<I(M)>`. This will also be the same URI as the named graph that `<I(M)>` is in.
* Models should be retrievable directly by their name, optionally including their inferences, if any exist.
* A Model that has no inferences can be later inferred.
* If model `<M>` updates, then if there exists a `<I(M)>`, then inferences should be done `<M>` and the contents of `<I(M)>` updated.
* Commonly, in a query, you'll want to make a query making use of inferences in `<I(M)>`, but returning results in `<M>`. This should be an automatic option.
* Also commonly, you'll want to query on Ontology Closures, `<Cl(O)>`, using the data there, but still refering to terms only directly referred to in `<M>`. Again, this should be an automatic option.

### Ontology

* Ontology data will all go into the same named graph. There will be only ontolgy named graph, denoted here as `<O>`
* Often relation closures on terms may want be precomputed. These closure triples will be put in a separate named graph, `<Cl(O)>`.
* Updates to the Ontology should trigger updates to `<Cl(O)>`

### Meta
* A manifest of all Models in the system, called `<Meta>`
  * What type of Model it is (Model, Inferred, Ontology, Closures)
  * What other model(s) it links to/from (`inferredFrom` etc)

Example manifest entry saying that the named graph <http://www.example.org/my_graph> is a Model:
```ttl
@prefix : <http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/> .
<http://www.example.org/my_graph> a :Model .

```

### Meta Ontology
* The Ontology used to define the terms used in `<Meta>`. This will be denoted `<MetaOnt>`


## Base Features

* Add model to a graph (from TTL file)
  * Model URI and named graph URI are synchronized?
  * A special meta graph holding the relationships between graphs
  * `<M> a :Model .`

## REST

### Communication

Some communication will be directly in TTL/N-tuple format for directly transferring ontologies around. And maybe some kind of simple CSV or table like format for sending sparql results.

But for communication to web clients, or other server clients, for the most part we should communicate with JSON-LD, with a `@context` defined here as well. Additionally, we should provide JSON schemas for all the JSON resources.

### Resource `/graph`
* `GET`: Lists all graphs. This should use `<Meta>`:
  * `?type=<graph type>`, one of `ontology`, `closure`, `model`, or `inferred`

```sparql
SELECT ?g ?p ?o WHERE {
   GRAPH :Meta {
      ?g ?p ?o
   }
}
```

```json
{
   "@context": "http://www.purl.org/dougli1sqrd/models/janus-oxide/meta/context.json",
   "graphs": [
      {
         "@id": <named graph URI>,
         "@type": "Ontology", // etc
         "url": <url to find this graph>
      },
      {
         "@id": <model named graph URI>,
         "@type": "Model",
         "hasInferencesAt": <Inferred Graph URI>,
         "url": <url to find this graph>
      },
      {
         "@id": <Inferred Graph URI>,
         "@type": "Inferred",
         "inferredFrom": <model named graph URI>,
         "url": <url to find this graph>
      }
      // ...etc
   ]
}
```

* `PUT`: Add a new model to the triplestore.
  * `?type=<Graph Type>`, one of `ontology`, `closure`, `model`, or `inferred`. This will determine what type of graph this should be. By default, this will be `model`.

   Data should be in Turtle, RDF/XML, etc (anything that oxigraph accepts).

   Data should be examined for the `?foo a owl:Ontology` and the named graph should be made using the URI of `?foo`.

   The named graph URI could also be found using the turtle `@base` keyword

   If there are no `Ontology` or `@base` URIs to be found, providing a `?model-id=<URI>` to the request will use the given `<URI>` instead.

   It should be considered invalid to have a model without a given named graph, so a `400` HTTP response code.

   The RDF data should be placed in a named graph with URI found above. Then, in `<Meta>` a new entry should be added:

   ```turtle
   <NamedGraph> a :Model .
   ```

### Resource `/graph/<uri>`

* `GET`: Get the contents of the named graph at `<uri>`
   * `?format=<type>` where `<type>` is an RDF format which will create and download a file of that format.

### Resource