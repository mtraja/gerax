import { request } from './config';

export async function listarMatriculas() {
  return request('/matriculas');
}

export async function criarMatricula(dados) {
  return request('/matriculas', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function deletarMatricula(id) {
  return request(`/matriculas/${id}`, {
    method: 'DELETE',
  });
}
