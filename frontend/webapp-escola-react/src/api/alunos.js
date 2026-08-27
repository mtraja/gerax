import { request } from './config';

export async function listarAlunos() {
  return request('/alunos');
}

export async function criarAluno(dados) {
  return request('/alunos', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function atualizarAluno(id, dados) {
  return request(`/alunos/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dados),
  });
}

export async function deletarAluno(id) {
  return request(`/alunos/${id}`, {
    method: 'DELETE',
  });
}
