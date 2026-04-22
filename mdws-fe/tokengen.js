const jwt = require('jsonwebtoken')
const fs = require('fs')

// Load your private key (must match the public.pem on your server)
const privateKey = fs.readFileSync('private.pem')

const payload = {
  sub: 'user123',
  name: 'John Doe',
  iat: Math.floor(Date.now() / 1000),
  exp: Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 30, // 1 hour expiration
}

const token = jwt.sign(payload, privateKey, { algorithm: 'RS256' })
console.log('Your Token:', token)
