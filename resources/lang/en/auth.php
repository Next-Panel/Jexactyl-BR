<?php

return [
    'sign_in' => 'Entrar',
    'go_to_login' => 'Vá para Login',
    'failed' => 'Nenhuma conta que corresponda a essas credenciais poderia ser encontrada.',

    'forgot_password' => [
        'label' => 'Esqueceu a senha?',
        'label_help' => 'Digite o endereço de E-mail da sua conta para receber instruções sobre a redefinição de sua senha.',
        'button' => 'Recuperar conta',
    ],

    'reset_password' => [
        'button' => 'Resetar e Entrar',
    ],

    'two_factor' => [
        'label' => 'Token de 2 fatores',
        'label_help' => 'Esta conta requer uma segunda camada de autenticação para continuar. Digite o código gerado pelo seu dispositivo para completar este login.',
        'checkpoint_failed' => 'O token de autenticação de dois fatores era inválido.',
    ],

    'throttle' => 'Muitas tentativas de login. Por favor, tente novamente em :seconds segundos.',
    'password_requirements' => 'A senha deve ter pelo menos 8 caracteres de comprimento e deve ser exclusiva deste site.',
    '2fa_must_be_enabled' => 'O administrador exigiu que a autenticação de 2 fatores seja habilitada para sua conta para usar o Painel.',
];
