const { blue, zinc, cyan } = require('tailwindcss/colors');

module.exports = {
"content":["./resources/scripts/**/*.{js,ts,tsx}"],"theme":{
"extend":{
"backgroundImage":{
"storeone":"url('https:
                'storetwo': "url('https:
                'storethree': "url('https:},"colors":{
"black":"#000","primary":blue,"neutral":zinc,"gray":zinc,"cyan":cyan},"fontSize":{
"2xs":"0.625rem"},"transitionDuration":{
"250":"250ms"},"borderColor":theme => ({
                default: theme('colors.neutral.400', 'currentColor'),
            })}},"plugins":[require('@tailwindcss/line-clamp'),require('@tailwindcss/forms')({
            strategy: 'class',
        })]}
