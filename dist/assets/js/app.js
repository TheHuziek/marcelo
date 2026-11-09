document.addEventListener("DOMContentLoaded", () => {
    fetch('#campus-seleccionado').addEventListener('change', (event) => {
        const selectedCampus = event.target.value;
        cambiarCampus(selectedCampus);
    });
});