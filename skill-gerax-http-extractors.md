Implemente o extractor `Headers` no Gerax.

Objetivo:
Criar um extractor que disponibilize todos os headers HTTP da requisição
ao handler.

API pública desejada:

async fn handler(
    Headers(headers): Headers
) {
    let authorization = headers.get("Authorization");
}

Requisitos:

1. Criar o tipo:

pub struct Headers(/* tipo apropriado */);

2. O extractor deve obter os headers diretamente de:

ctx.request().headers()

3. O extractor NÃO deve:
   - receber nome de header;
   - converter valores usando FromStr;
   - desserializar headers;
   - implementar a lógica de `Header<T>`;
   - criar uma cópia dos headers sem necessidade.

4. Deve preservar a API/tipo de headers utilizado pelo servidor HTTP
   subjacente.

5. Verifique primeiro como `Context`, `Request` e os extractors existentes
   do Gerax estão implementados.

6. Siga exatamente o mesmo padrão utilizado pelos outros extractors
   existentes, incluindo:
   - trait de extração;
   - tipo de erro;
   - lifetimes;
   - ownership/borrowing;
   - organização dos módulos;
   - exports/re-exports.

7. Determine se `Headers` deve possuir os headers por valor, referência ou
   outra representação, considerando o lifetime do extractor e o contrato
   atual do Gerax.

8. Adicione testes para:
   - requisição contendo vários headers;
   - acesso a um header existente;
   - ausência de um header;
   - preservação dos valores dos headers.

9. Atualize a documentação com um exemplo realista:

async fn handler(
    Headers(headers): Headers
) {
    if let Some(value) = headers.get("Authorization") {
        // ...
    }
}

10. Não altere o comportamento dos extractors existentes.

11. Execute:

cargo fmt --all
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features

Antes de implementar, inspecione o código existente e explique brevemente
qual tipo concreto representa os headers HTTP no Gerax e por que essa
representação foi escolhida.

Ao finalizar, informe:
- arquivos criados/alterados;
- implementação realizada;
- testes adicionados;
- resultado dos comandos de validação.