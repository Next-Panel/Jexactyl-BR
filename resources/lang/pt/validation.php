<?php

return [
    /*
    |--------------------------------------------------------------------------
    | Validation Language Lines
    |--------------------------------------------------------------------------
    |
    | The following language lines contain the default error messages used by
    | the validator class. Some of these rules have multiple versions such
    | as the size rules. Feel free to tweak each of these messages here.
    |
    */

    'accepted' => 'O :attribute deve ser aceito.',
    'active_url' => 'O :attribute não é uma URL válida.',
    'after' => 'O :attribute deve ser uma data depois :date.',
    'after_or_equal' => 'O :attribute deve ser uma data depois ou igual a :date.',
    'alpha' => 'O :attribute só pode conter letras.',
    'alpha_dash' => 'O :attribute só pode conter letras, números e traços.',
    'alpha_num' => 'The :attribute may only contain letters and numbers.',
    'array' => 'O :attribute deve ser uma matriz.',
    'before' => 'O :attribute deve ser uma data antes :date.',
    'before_or_equal' => 'O :attribute deve ser uma data antes ou igual a :date.',
    'between' => [
        'numeric' => 'O :attribute deve ser entre :min e :max.',
        'file' => 'O :attribute deve ser entre :min e :max kilobytes.',
        'string' => 'The :attribute email must be between :min and :max characters.',
        'array' => 'O :attribute deve ter entre :min e :max items.',
    ],
    'boolean' => 'O :attribute campo deve ser verdadeiro ou falso.',
    'confirmed' => 'The :attribute confirmation does not match.',
    'date' => 'O :attribute is not a valid date.',
    'date_format' => 'O :attribute não corresponde ao formato :format.',
    'different' => 'O :attribute e :other deve ser diferente.',
    'digits' => 'O :attribute deve ser :digits digits.',
    'digits_between' => 'O :attribute deve ser entre :min e :max digitos.',
    'dimensions' => 'O :attribute tem dimensões de imagem inválidas.',
    'distinct' => 'O :attribute campo tem um valor duplicado.',
    'email' => 'O :attribute deve ser um endereço de e-mail válido.',
    'exists' => 'The selected :attribute is invalid.',
    'file' => 'O :attribute deve ser um arquivo.',
    'filled' => 'The :attribute field is required.',
    'image' => 'O :attribute deve ser uma imagem.',
    'in' => 'The selected :attribute is invalid.',
    'in_array' => 'O :attribute campo não existe em :other.',
    'integer' => 'O :attribute deve ser um inteiro.',
    'ip' => 'O :attribute deve ser um endereço IP válido.',
    'json' => 'O :attribute deve ser uma sequência JSON válida.',
    'max' => [
        'numeric' => 'O :attribute não pode ser maior do que :max.',
        'file' => 'O :attribute não pode ser maior do que :max kilobytes.',
        'string' => 'O :attribute não pode ser maior do que :max characters.',
        'array' => 'The :attribute may not have more than :max items.',
    ],
    'mimes' => 'O :attribute deve ser um arquivo de tipo: :values.',
    'mimetypes' => 'O :attribute deve ser um arquivo de tipo: :values.',
    'min' => [
        'numeric' => 'O :attribute deve ser, pelo menos, :min.',
        'file' => 'O :attribute deve ser, pelo menos, :min kilobytes.',
        'string' => 'O :attribute deve ser, pelo menos, :min Caracteres.',
        'array' => 'O :attribute deve ter, pelo menos, :min items.',
    ],
    'not_in' => 'O :attribute is invalid.',
    'numeric' => 'O :attribute deve ser um número.',
    'present' => 'O :attribute campo deve estar presente.',
    'regex' => 'O :attribute o formato é inválido.',
    'required' => 'The :attribute field is required.',
    'required_if' => 'O :attribute campo é necessário quando :other é :value.',
    'required_unless' => 'O :attribute campo é necessário a menos que :other está em :values.',
    'required_with' => 'O :attribute campo é necessário quando :values está presente.',
    'required_with_all' => 'O :attribute campo é necessário quando :values está presente.',
    'required_without' => 'O :attribute campo é necessário quando :values não está presente.',
    'required_without_all' => 'O :attribute campo é necessário quando nenhum dos :values estão presentes.',
    'same' => 'O :attribute e :other deve corresponder.',
    'size' => [
        'numeric' => 'O :attribute deve ser :size.',
        'file' => 'O :attribute deve ser :size kilobytes.',
        'string' => 'O :attribute deve ser :size characters.',
        'array' => 'O :attribute deve conter :size items.',
    ],
    'string' => 'O :attribute deve ser uma corda.',
    'timezone' => 'O :attribute deve ser uma zona válida.',
    'unique' => 'O :attribute já foi tomada.',
    'uploaded' => 'O :attribute não fez upload.',
    'url' => 'O :attribute o formato é inválido.',

    /*
    |--------------------------------------------------------------------------
    | Custom Validation Attributes
    |--------------------------------------------------------------------------
    |
    | The following language lines are used to swap attribute place-holders
    | with something more reader friendly such as E-Mail Address instead
    | of "email". This simply helps us make messages a little cleaner.
    |
    */

    'attributes' => [],

    // Internal validation logic for Pterodactyl
    'internal' => [
        'variable_value' => ':env variable',
        'invalid_password' => 'The password provided was invalid for this account.',
    ],
];
