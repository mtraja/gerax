import { request } from './config';

export async function listarTurmas() {
  return request('/turmas');
}

export async function criarTurma(dados) {
  return request('/turmas', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function atualizarTurma(id, dados) {
  return request(`/turmas/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dados),
  });
}

export async function deletarTurma(id) {
  return request(`/turmas/${id}`, {
    method: 'DELETE',
  });
}
