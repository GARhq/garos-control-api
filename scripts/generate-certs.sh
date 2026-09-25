#!/usr/bin/env bash
set -euo pipefail

# Cria certificados locais autoassinados para rodar a API em HTTPS durante o dev.
# Também gera um par de chaves RSA para assinar os JWTs.

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CERTS_DIR="$DIR/certs"

mkdir -p "$CERTS_DIR"

echo "🔐 Gerando chaves para o JWT (RS256)..."
# Private key
openssl genpkey -algorithm RSA -out "$CERTS_DIR/jwt_private.pem" -pkeyopt rsa_keygen_bits:2048
# Public key
openssl rsa -pubout -in "$CERTS_DIR/jwt_private.pem" -out "$CERTS_DIR/jwt_public.pem"
echo "  -> jwt_private.pem"
echo "  -> jwt_public.pem"

echo "🔐 Gerando certificado HTTPS self-signed..."
cat > "$CERTS_DIR/san.cnf" <<EOF
[req]
default_bits       = 2048
distinguished_name = req_distinguished_name
req_extensions     = req_ext
x509_extensions    = v3_ca # The extensions to add to the self signed cert
prompt             = no

[req_distinguished_name]
CN = garos.kryonix.local

[req_ext]
subjectAltName = @alt_names

[v3_ca]
subjectAltName = @alt_names
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth

[alt_names]
DNS.1   = localhost
DNS.2   = garos.kryonix.local
IP.1    = 127.0.0.1
IP.2    = 0.0.0.0
EOF

openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout "$CERTS_DIR/server.key" \
  -out "$CERTS_DIR/server.crt" \
  -config "$CERTS_DIR/san.cnf"

rm "$CERTS_DIR/san.cnf"

echo "  -> server.key"
echo "  -> server.crt"

echo ""
echo "✅ Pronto! Para usar no ambiente de dev, você pode rodar:"
echo "export GAROS_AUTH__JWT_PRIVATE_KEY_PATH=$CERTS_DIR/jwt_private.pem"
echo "export GAROS_AUTH__JWT_PUBLIC_KEY_PATH=$CERTS_DIR/jwt_public.pem"
echo ""
echo "(Nota: O Axum em garos-control-api precisará ser configurado para usar o server.key/.crt se quiser servir HTTPS nativamente,"
echo " ou você pode colocar um nginx/caddy na frente.)"
