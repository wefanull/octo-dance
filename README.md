# GitHub Party Explorer 🕺

Aplicación web interactiva que consulta información de usuarios y repositorios de GitHub a través de su API GraphQL, utilizando una arquitectura desacoplada con **Go**, **Rust** y **Frontend web**.

---

## 🏗️ Arquitectura del Proyecto

El sistema está dividido en dos servicios principales y una interfaz web:

```text
Navegador Web (Frontend)
       │
       ▼ (HTTP :8080)
Gateway en Go (Reverse Proxy & Servidor Estático)
       │
       ▼ (HTTP :3000)
API en Rust (Axum + Reqwest)
       │
       ▼ (GraphQL API)
GitHub
```

* **Frontend (`gateway/static`)**: Interfaz web en HTML5, CSS y Vanilla JS con animaciones, historial de búsquedas y visualizador JSON.
* **Gateway (`gateway`)**: Servidor web y proxy inverso en Go que valida entradas y redirige las solicitudes a la API interna.
* **API (`api`)**: Microservicio asíncrono en Rust con Axum que ejecuta consultas GraphQL directas hacia la API de GitHub.

---

## 📋 Requisitos Previos

* [Rust & Cargo](https://www.rust-lang.org/tools/install)
* [Go](https://go.dev/dl/) (versión 1.22 o superior)
* Un **Personal Access Token** de GitHub ([Crear token en GitHub](https://github.com/settings/tokens))

---

## ⚙️ Configuración

1. **Clonar el repositorio**:
   ```bash
   git clone <url-del-repositorio>
   cd Clase3
   ```

2. **Configurar el token de GitHub en la API de Rust**:
   Copia el archivo de ejemplo y agrega tu token:
   ```bash
   cp api/.env.example api/.env
   ```
   Abre `api/.env` y coloca tu token:
   ```env
   GITHUB_TOKEN=ghp_tuTokenDeGitHubAqui
   ```

---

## 🚀 Ejecución

Para iniciar el proyecto completo, abre dos terminales:

### 1. Iniciar la API de Rust (Puerto 3000)
```bash
cd api
cargo run
```

### 2. Iniciar el Gateway de Go (Puerto 8080)
```bash
cd gateway
go run main.go
```

### 3. Abrir en el navegador
Ingresa a:
```text
http://localhost:8080
```

---

## 🔌 Endpoints de la API

* `GET /api/user/{username}`: Devuelve el nombre, biografía, avatar, fecha y User-Agent del usuario.
* `GET /api/repos/{username}`: Devuelve la lista de repositorios públicos del usuario (hasta 50).

---

## 📂 Estructura de Carpetas

```text
.
├── api/                  # Microservicio en Rust
│   ├── src/
│   │   └── main.rs       # Endpoints Axum y cliente GraphQL
│   ├── .env.example      # Plantilla de variables de entorno
│   ├── Cargo.toml
│   └── Cargo.lock
├── gateway/              # Gateway y Frontend en Go
│   ├── static/
│   │   └── index.html    # Aplicación web estática
│   ├── main.go           # Proxy inverso y servidor HTTP
│   └── go.mod
├── .gitignore
└── README.md
```
