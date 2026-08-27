import { Link, NavLink } from 'react-router-dom';
import { useState } from 'react';
import './Layout.css';

const themes = [
  { value: 'indigo', label: 'Claro índigo' },
  { value: 'aurora', label: 'Claro aurora' },
  { value: 'paper', label: 'Claro areia' },
  { value: 'midnight', label: 'Escuro meia-noite' },
  { value: 'forest', label: 'Escuro floresta' },
  { value: 'black', label: 'Escuro pleno' },
];

export default function Layout({ children }) {
  const [theme, setTheme] = useState(() => localStorage.getItem('gerax-theme') || 'indigo');

  function alterarTema(event) {
    const nextTheme = event.target.value;
    setTheme(nextTheme);
    localStorage.setItem('gerax-theme', nextTheme);
  }

  return (
    <div className="app" data-theme={theme}>
      <header className="app-header">
        <Link className="brand" to="/">
          <span className="brand-mark">GE</span>
          Gerax Escola
        </Link>
        <nav aria-label="Navegação principal">
          <NavLink to="/" end>Início</NavLink>
          <NavLink to="/alunos">Alunos</NavLink>
          <NavLink to="/professores">Professores</NavLink>
          <NavLink to="/turmas">Turmas</NavLink>
          <NavLink to="/matriculas">Matrículas</NavLink>
        </nav>
        <label className="theme-picker">
          <span>Tema</span>
          <select value={theme} onChange={alterarTema} aria-label="Escolher tema">
            {themes.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
          </select>
        </label>
      </header>
      <main className="content">{children}</main>
    </div>
  );
}
