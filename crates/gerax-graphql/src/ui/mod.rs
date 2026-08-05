pub struct GraphiQL {
    endpoint: String,
}

impl GraphiQL {
    /// Cria uma nova instância de GraphiQL com o endpoint especificado.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    /// Renderiza o HTML do GraphiQL.
    pub fn render(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>GraphiQL</title>
    <meta charset="utf-8"/>
    <style>
        body {{ margin: 0; overflow: hidden; }}
        #graphiql {{ height: 100vh; }}
    </style>
</head>
<body>
    <div id="graphiql">Loading...</div>
    <script src="https://unpkg.com/graphiql/graphiql.min.js"></script>
    <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css"/>
    <script>
        const root = document.getElementById('graphiql');
        const fetcher = GraphiQL.createFetcher({{ url: '{}' }});
        ReactDOM.render(
            React.createElement(GraphiQL, {{ fetcher: fetcher }}),
            root
        );
    </script>
</body>
</html>"#,
            self.endpoint
        )
    }
}

/// Interface GraphQL Playground para GraphQL.
///
/// Fornece uma interface web interativa para
/// executar queries e mutations GraphQL.
///
/// ## Exemplo
///
/// ```rust
/// use gerax_graphql::Playground;
///
/// let playground = Playground::new("/graphql");
/// let html = playground.render();
/// ```
pub struct Playground {
    endpoint: String,
}

impl Playground {
    /// Cria uma nova instância de Playground com o endpoint especificado.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    /// Renderiza o HTML do Playground.
    pub fn render(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>GraphQL Playground</title>
    <meta charset="utf-8"/>
    <style>
        body {{ margin: 0; overflow: hidden; }}
        #playground {{ height: 100vh; }}
    </style>
</head>
<body>
    <div id="playground">Loading...</div>
    <script src="https://unpkg.com/graphql-playground-react@1.7/build/static/js/index.js"></script>
    <link rel="stylesheet" href="https://unpkg.com/graphql-playground-react@1.7/build/static/css/index.css"/>
    <script>
        const root = document.getElementById('playground');
        const endpoint = '{}';
        GraphQLPlayground.render(root, {{ endpoint: endpoint }});
    </script>
</body>
</html>"#,
            self.endpoint
        )
    }
}
