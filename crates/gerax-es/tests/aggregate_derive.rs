//! Teste de integração do derive `#[derive(Aggregate)]`.

use gerax_es::{Aggregate, AggregateApplier, Version};

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

#[derive(Debug, Aggregate)]
#[aggregate(aggregate_type = "Aluno")]
struct Student {
    id: String,
    name: String,
    version: Version,
    created: bool,
    pending: Vec<StudentEvent>,
}

impl AggregateApplier for Student {
    type Event = StudentEvent;
    type Error = StudentError;

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
}

#[test]
fn derive_gera_aggregate_completo() {
    let mut student = Student::empty("student-1".into());

    assert_eq!(student.id(), "student-1");
    assert_eq!(Student::aggregate_type(), "Aluno");
    assert_eq!(student.version(), Version::initial());
    assert_eq!(student.name, "");
    assert!(!student.created);
    assert!(student.take_events().is_empty());
}

#[test]
fn derive_raise_acumula_pendentes_e_aplica_estado() {
    let mut student = Student::empty("student-1".into());

    student
        .raise(StudentEvent::Created { name: "Ana".into() })
        .unwrap();
    student
        .raise(StudentEvent::Renamed { name: "Bia".into() })
        .unwrap();

    assert_eq!(student.name, "Bia");
    assert_eq!(student.version(), Version::new(2));
    assert_eq!(student.take_events().len(), 2);
    assert!(student.take_events().is_empty());
}

#[test]
fn derive_apply_eh_usado_na_reidratacao() {
    let mut student = Student::empty("student-1".into());
    student
        .apply(&StudentEvent::Created { name: "Ana".into() })
        .unwrap();

    assert_eq!(student.name, "Ana");
    assert_eq!(student.version(), Version::new(1));
    assert!(student.take_events().is_empty());
}

#[test]
fn derive_repassa_erros_de_dominio() {
    let mut student = Student::empty("student-1".into());
    student
        .raise(StudentEvent::Created { name: "Ana".into() })
        .unwrap();

    let err = student
        .raise(StudentEvent::Created {
            name: "Outra".into(),
        })
        .unwrap_err();

    assert!(matches!(err, StudentError::AlreadyCreated));
    assert_eq!(student.name, "Ana");
    assert_eq!(student.version(), Version::new(1));
}
