import { request } from './config';

export async function listarProfessores() {
  return request('/professores');
}

export async function criarProfessor(dados) {
  return request('/professores', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function atualizarProfessor(id, dados) {
  return request(`/professores/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dados),
  });
}

export async function deletarProfessor(id) {
  return request(`/professores/${id}`, {
    method: 'DELETE',
  });
}
