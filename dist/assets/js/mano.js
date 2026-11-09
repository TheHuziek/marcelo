function cambiarCampus(nombreCampus) {
    // Seleccionamos el elemento span por su id y cambiamos su contenido
    document.getElementById('campus-seleccionado').innerText = nombreCampus;
}
try {
    // Usamos una ruta relativa porque está en el mismo servidor
    const respuesta = await fetch('/api/v1/publicaciones');

    if (!respuesta.ok) {
        throw new Error(`Error en la petición: ${respuesta.status}`);
    }

    const datos = await respuesta.json();
} catch (error) {
    console.error('Hubo un problema:', error);
}