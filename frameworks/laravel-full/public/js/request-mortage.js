document.addEventListener('DOMContentLoaded', () => {
    const dpOptions = document.querySelectorAll('.dp-option');
    const housePrice = parseFloat(document.getElementById('housePrice').value) || 0;
    const interestRate = parseFloat(document.getElementById('interestRate').value) || 0;
    const durationYears = parseFloat(document.getElementById('durationYears').value) || 0;
    const dpPercentageInput = document.getElementById('dp-percentage');

    dpOptions.forEach((option) => {
        option.addEventListener('click', (event) => {
            event.preventDefault();

            // Remove bg-tedja-black and text-white from all options
            dpOptions.forEach((opt) => {
                opt.classList.remove('bg-tedja-black', 'text-white');
                opt.classList.add('bg-white', 'text-black'); // Optional: Revert to default styles
            });

            // Add bg-tedja-black and text-white to the clicked option
            option.classList.add('bg-tedja-black', 'text-white');
            option.classList.remove('bg-white', 'text-black'); // Optional: Remove default styles

            // Get percentage from data attribute
            const percentage = parseFloat(option.getAttribute('data-percentage'));

            // Ensure percentage is valid
            if (isNaN(percentage) || isNaN(housePrice) || isNaN(interestRate) || isNaN(durationYears)) {
                console.error('Invalid calculation inputs:', {
                    percentage,
                    housePrice,
                    interestRate,
                    durationYears,
                });
                return;
            }

            // Update hidden input value
            dpPercentageInput.value = percentage;

            // Calculate details
            const downPaymentAmount = housePrice * (percentage / 100);
            const loanAmount = housePrice - downPaymentAmount;
            const monthlyInterestRate = interestRate / 100 / 12; // Convert annual rate to monthly rate
            const totalPayments = durationYears * 12;

            // Amortization formula for monthly payment
            const numerator = loanAmount * monthlyInterestRate * Math.pow(1 + monthlyInterestRate, totalPayments);
            const denominator = Math.pow(1 + monthlyInterestRate, totalPayments) - 1;
            const monthlyPayment = numerator / denominator;

            const monthlyPPN = monthlyPayment * 0.11; // Tax (11% of monthly payment)
            const totalLoanWithInterest = monthlyPayment * totalPayments; // Total loan repayment

            // Update UI
            document.getElementById('down-payment-percent').textContent = `${percentage}%`;
            document.getElementById('down-payment-amount').textContent =
                `Rp ${Math.round(downPaymentAmount).toLocaleString('id')}`;
            document.getElementById('total-loan').textContent = `Rp ${Math.round(loanAmount).toLocaleString('id')}`;
            document.getElementById('monthly-payment').textContent =
                `Rp ${Math.round(monthlyPayment).toLocaleString('id')}`;
            document.getElementById('monthly-ppn').textContent = `Rp ${Math.round(monthlyPPN).toLocaleString('id')}`;
            document.getElementById('loan-with-interest').textContent =
                `Rp ${Math.round(totalLoanWithInterest).toLocaleString('id')}`;
        });
    });
});
