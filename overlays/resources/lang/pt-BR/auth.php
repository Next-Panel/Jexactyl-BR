<?php

// Tradução pt-BR das strings de autenticação do Jexactyl.
// Onda 01 da migração: a tradução entra aos poucos, arquivo por arquivo.

return [
    'sign_in' => 'Entrar',
    'go_to_login' => 'Ir para o login',
    'failed' => 'Nenhuma conta corresponde a essas credenciais.',

    'forgot_password' => [
        'label' => 'Esqueceu a senha?',
        'label_help' => 'Informe o e-mail da sua conta para receber as instruções de redefinição de senha.',
        'button' => 'Recuperar conta',
    ],

    'reset_password' => [
        'button' => 'Redefinir e entrar',
    ],

    'two_factor' => [
        'label' => 'Token de dois fatores',
        'label_help' => 'Esta conta exige uma segunda camada de autenticação para continuar. Informe o código gerado pelo seu dispositivo para concluir o login.',
        'checkpoint_failed' => 'O token de autenticação de dois fatores é inválido.',
    ],

    'throttle' => 'Muitas tentativas de login. Tente novamente em :seconds segundos.',
    'password_requirements' => 'A senha deve ter no mínimo 8 caracteres e não deve ser reutilizada em outros sites.',
    '2fa_must_be_enabled' => 'O administrador exigiu que a autenticação de dois fatores esteja ativada na sua conta para usar o Painel.',
];
