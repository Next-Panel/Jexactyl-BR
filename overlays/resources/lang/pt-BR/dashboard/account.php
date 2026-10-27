<?php

return [
    'email' => [
        'title' => 'Atualize seu e-mail',
        'updated' => 'Seu endereço de e-mail foi atualizado.',
    ],
    'password' => [
        'title' => 'Altere sua senha',
        'requirements' => 'Sua nova senha deve ter pelo menos 8 caracteres.',
        'updated' => 'Sua senha foi atualizada.',
    ],
    'two_factor' => [
        'button' => 'Configurar autenticação de dois fatores',
        'disabled' => 'A autenticação de dois fatores foi desativada na sua conta. Você não precisará mais informar um token ao entrar.',
        'enabled' => 'A autenticação de dois fatores foi ativada na sua conta! A partir de agora, ao entrar, você precisará informar o código gerado pelo seu dispositivo.',
        'invalid' => 'O token informado é inválido.',
        'setup' => [
            'title' => 'Configurar autenticação de dois fatores',
            'help' => 'Não consegue escanear o código? Insira o código abaixo no seu aplicativo:',
            'field' => 'Informe o token',
        ],
        'disable' => [
            'title' => 'Desativar autenticação de dois fatores',
            'field' => 'Informe o token',
        ],
    ],
];
