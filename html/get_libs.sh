#!/bin/sh

# Create the library directory
mkdir -p lib

echo " Fetching theater dependencies..."

# Download files using curl
# -L follows redirects, -o specifies the output filename
curl -L "https://unpkg.com/@babel/standalone/babel.min.js" -o lib/babel.min.js
curl -L "https://cdn.tailwindcss.com" -o lib/tailwind.js
curl -L "https://unpkg.com/react@18/umd/react.production.min.js" -o lib/react.min.js
curl -L "https://unpkg.com/react-dom@18/umd/react-dom.production.min.js" -o lib/react-dom.min.js


echo "Done! Files are in the /lib folder."
ls -l lib