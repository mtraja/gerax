import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import Alunos from './pages/Alunos';
import CriarAluno from './pages/CriarAluno';
import Professores from './pages/Professores';
import CriarProfessor from './pages/CriarProfessor';
import Turmas from './pages/Turmas';
import CriarTurma from './pages/CriarTurma';
import Matriculas from './pages/Matriculas';
import CriarMatricula from './pages/CriarMatricula';

export default function App() {
  return (
    <Router>
      <Layout>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/alunos" element={<Alunos />} />
          <Route path="/alunos/criar" element={<CriarAluno />} />
          <Route path="/professores" element={<Professores />} />
          <Route path="/professores/criar" element={<CriarProfessor />} />
          <Route path="/turmas" element={<Turmas />} />
          <Route path="/turmas/criar" element={<CriarTurma />} />
          <Route path="/matriculas" element={<Matriculas />} />
          <Route path="/matriculas/criar" element={<CriarMatricula />} />
        </Routes>
      </Layout>
    </Router>
  );
}
