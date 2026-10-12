package main

import (
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"regexp"
	"strings"
	"time"
)

const (
	goPort = "8080"
)

// Expresión regular para usernames de GitHub (1-39 caracteres alfanuméricos y guiones no consecutivos)
var validUsername = regexp.MustCompile(`^[a-zA-Z0-9](?:[a-zA-Z0-9]|-(?=[a-zA-Z0-9])){0,38}$`)

// Cliente HTTP compartido con timeout para prevenir consumo infinito de recursos
var httpClient = &http.Client{
	Timeout: 10 * time.Second,
}

// Cabeceras Hop-by-Hop que no deben ser reenviadas por un proxy
var hopByHopHeaders = map[string]bool{
	"connection":          true,
	"keep-alive":          true,
	"proxy-authenticate":  true,
	"proxy-authorization": true,
	"te":                  true,
	"trailers":            true,
	"transfer-encoding":   true,
	"upgrade":             true,
}

func main() {
	// URL de la API de Rust
	rustTarget := os.Getenv("RUST_API_URL")
	if rustTarget == "" {
		rustTarget = "http://localhost:3000"
	}

	// Servir archivos estáticos del frontend
	fs := http.FileServer(http.Dir("./static"))
	http.Handle("/", fs)

	// Proxy endpoints
	http.HandleFunc("/api/user/", handleProxy("user", rustTarget))
	http.HandleFunc("/api/repos/", handleProxy("repos", rustTarget))

	server := &http.Server{
		Addr:         "0.0.0.0:" + goPort,
		ReadTimeout:  5 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	fmt.Printf("Servidor Go corriendo en http://0.0.0.0:%s\n", goPort)
	fmt.Printf("Apuntando a la API de Rust en: %s\n", rustTarget)

	log.Fatal(server.ListenAndServe())
}

func handleProxy(resource string, rustTarget string) http.HandlerFunc {
	prefix := "/api/" + resource + "/"
	return func(w http.ResponseWriter, r *http.Request) {
		if !strings.HasPrefix(r.URL.Path, prefix) {
			http.Error(w, "Ruta no encontrada", http.StatusNotFound)
			return
		}

		username := r.URL.Path[len(prefix):]
		// Eliminar posibles slashes finales accidentales
		username = strings.Trim(username, "/")

		if username == "" {
			http.Error(w, "Nombre de usuario requerido", http.StatusBadRequest)
			return
		}

		// Validación contra Path Traversal y caracteres no permitidos
		if !validUsername.MatchString(username) {
			http.Error(w, "Formato de nombre de usuario inválido", http.StatusBadRequest)
			return
		}

		// Construir URL hacia la API de Rust
		rustUrl := fmt.Sprintf("%s/%s/%s", rustTarget, resource, username)

		// Usar el contexto de la petición entrante para cancelar la llamada si el cliente aborta
		req, err := http.NewRequestWithContext(r.Context(), "GET", rustUrl, nil)
		if err != nil {
			http.Error(w, "Error creando petición interna", http.StatusInternalServerError)
			return
		}

		if clientUA := r.Header.Get("User-Agent"); clientUA != "" {
			req.Header.Set("User-Agent", clientUA)
		}

		resp, err := httpClient.Do(req)
		if err != nil {
			http.Error(w, "La API de Rust no responde en "+rustTarget, http.StatusServiceUnavailable)
			return
		}
		defer resp.Body.Close()

		// Copiar cabeceras ignorando las cabeceras Hop-by-Hop
		for name, values := range resp.Header {
			if hopByHopHeaders[strings.ToLower(name)] {
				continue
			}
			for _, value := range values {
				w.Header().Add(name, value)
			}
		}

		w.WriteHeader(resp.StatusCode)
		io.Copy(w, resp.Body)
	}
}
