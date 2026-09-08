//! Raiz de Aggregate (Event Sourcing).

use crate::version::Version;

/// Raiz de um Aggregate no contexto de Event Sourcing.
///
/// Um Aggregate deve ser capaz de:
///
/// 1. ser criado vazio ([`Aggregate::empty`]);
/// 2. aplicar eventos históricos ([`Aggregate::apply`]);
/// 3. gerar novos eventos ([`Aggregate::raise`]);
/// 4. manter eventos pendentes ([`Aggregate::take_events`]);
/// 5. controlar sua versão ([`Aggregate::version`]).
///
/// Implementações concretas devem permanecer **síncronas** e não
/// conhecer infraestrutura (bancos de dados, bus async, HTTP, runtime
/// assíncrono etc.).
///
/// ## `apply` vs `raise`
///
/// - [`Aggregate::apply`] é usado para **eventos históricos**: altera o
///   estado, atualiza a versão e **não** adiciona o evento aos eventos
///   pendentes.
/// - [`Aggregate::raise`] é usado para **novos eventos**: aplica o
///   evento ao estado (`apply`) e o adiciona aos eventos pendentes.
pub trait Aggregate: Sized {
    /// Tipo do identificador do Aggregate.
    type Id;

    /// Tipo do Evento de domínio produzido pelo Aggregate.
    type Event;

    /// Tipo de erro produzido pelas operações do Aggregate.
    type Error;

    /// Nome canônico do tipo do Aggregate (ex.: `"Student"`).
    fn aggregate_type() -> &'static str;

    /// Cria um Aggregate vazio, sem eventos aplicados.
    fn empty(id: Self::Id) -> Self;

    /// Identificador do Aggregate.
    fn id(&self) -> &Self::Id;

    /// Versão atual do Aggregate no Event Stream.
    fn version(&self) -> Version;

    /// Aplica um evento ao estado do Aggregate.
    ///
    /// Deve alterar o estado interno, atualizar a versão e **não**
    /// adicionar o evento aos eventos pendentes.
    fn apply(&mut self, event: &Self::Event) -> Result<(), Self::Error>;

    /// Gera um novo evento.
    ///
    /// Comportamento conceitual:
    ///
    /// ```text
    /// raise(event)
    ///     ├── apply(event)        // atualiza o estado
    ///     └── pending.push(event) // registra como pendente
    /// ```
    fn raise(&mut self, event: Self::Event) -> Result<(), Self::Error>;

    /// Remove e retorna os eventos pendentes do Aggregate.
    fn take_events(&mut self) -> Vec<Self::Event>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
    enum StudentError {
        #[error("student already created")]
        AlreadyCreated,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum StudentEvent {
        Created { name: String },
        Renamed { name: String },
    }

    #[derive(Debug)]
    struct Student {
        id: String,
        name: String,
        version: Version,
        created: bool,
        pending: Vec<StudentEvent>,
    }

    impl Aggregate for Student {
        type Id = String;
        type Event = StudentEvent;
        type Error = StudentError;

        fn aggregate_type() -> &'static str {
            "Student"
        }

        fn empty(id: Self::Id) -> Self {
            Self {
                id,
                name: String::new(),
                version: Version::initial(),
                created: false,
                pending: Vec::new(),
            }
        }

        fn id(&self) -> &Self::Id {
            &self.id
        }

        fn version(&self) -> Version {
            self.version
        }

        fn apply(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
            match event {
                StudentEvent::Created { name } => {
                    if self.created {
                        return Err(StudentError::AlreadyCreated);
                    }
                    self.created = true;
                    self.name = name.clone();
                }
                StudentEvent::Renamed { name } => {
                    self.name = name.clone();
                }
            }
            self.version = self.version.next();
            Ok(())
        }

        fn raise(&mut self, event: Self::Event) -> Result<(), Self::Error> {
            self.apply(&event)?;
            self.pending.push(event);
            Ok(())
        }

        fn take_events(&mut self) -> Vec<Self::Event> {
            std::mem::take(&mut self.pending)
        }
    }

    #[test]
    fn empty_aggregate_eh_vazio_e_sem_versao() {
        let mut student = Student::empty("student-1".into());

        assert_eq!(student.id(), "student-1");
        assert_eq!(Student::aggregate_type(), "Student");
        assert_eq!(student.version(), Version::initial());
        assert!(student.take_events().is_empty());
    }

    #[test]
    fn apply_evento_historico_muda_estado_e_versao() {
        let mut student = Student::empty("student-1".into());

        student
            .apply(&StudentEvent::Created { name: "Ana".into() })
            .unwrap();

        assert_eq!(student.name, "Ana");
        assert_eq!(student.version(), Version::new(1));
        // apply não gera eventos pendentes.
        assert!(student.pending.is_empty());
    }

    #[test]
    fn apply_nao_adiciona_eventos_pendentes() {
        let mut student = Student::empty("student-1".into());

        student
            .apply(&StudentEvent::Created { name: "Ana".into() })
            .unwrap();
        student
            .apply(&StudentEvent::Renamed { name: "Bia".into() })
            .unwrap();

        assert_eq!(student.version(), Version::new(2));
        assert!(student.take_events().is_empty());
    }

    #[test]
    fn raise_gera_estado_e_evento_pendente() {
        let mut student = Student::empty("student-1".into());

        student
            .raise(StudentEvent::Created { name: "Ana".into() })
            .unwrap();

        assert_eq!(student.name, "Ana");
        assert_eq!(student.version(), Version::new(1));
        assert_eq!(student.pending.len(), 1);
        assert_eq!(
            student.pending[0],
            StudentEvent::Created { name: "Ana".into() }
        );
    }

    #[test]
    fn raise_acumula_eventos_pendentes() {
        let mut student = Student::empty("student-1".into());
        student
            .raise(StudentEvent::Created { name: "Ana".into() })
            .unwrap();
        student
            .raise(StudentEvent::Renamed { name: "Bia".into() })
            .unwrap();

        assert_eq!(student.pending.len(), 2);
        assert_eq!(student.version(), Version::new(2));
    }

    #[test]
    fn take_events_drena_os_pendentes() {
        let mut student = Student::empty("student-1".into());
        student
            .raise(StudentEvent::Created { name: "Ana".into() })
            .unwrap();
        student
            .raise(StudentEvent::Renamed { name: "Bia".into() })
            .unwrap();

        let events = student.take_events();

        assert_eq!(events.len(), 2);
        assert!(student.pending.is_empty());
    }

    #[test]
    fn apply_dobrado_de_criacao_rejeita() {
        let mut student = Student::empty("student-1".into());
        student
            .apply(&StudentEvent::Created { name: "Ana".into() })
            .unwrap();

        let err = student
            .apply(&StudentEvent::Created {
                name: "Outra".into(),
            })
            .unwrap_err();

        assert!(matches!(err, StudentError::AlreadyCreated));
        // estado e versão não foram alterados pelo evento rejeitado.
        assert_eq!(student.name, "Ana");
        assert_eq!(student.version(), Version::new(1));
    }

    #[test]
    fn raise_mantem_versao_coerente_entre_estado_e_pendentes() {
        let mut student = Student::empty("student-1".into());
        student
            .raise(StudentEvent::Created { name: "Ana".into() })
            .unwrap();

        let events = student.take_events();
        assert_eq!(events.len(), 1);

        // Após drenar, a versão permanece igual à posição no stream.
        assert_eq!(student.version(), Version::new(1));
    }
}
