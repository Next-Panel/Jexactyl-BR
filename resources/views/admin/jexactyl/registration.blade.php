@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'registration'])

@section('title')
    Configurações do Jexactyl
@endsection

@section('content-header')
    <h1>Registro de usuários<small>Definir as configurações para o registro de usuários no Jexactyl.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
@yield('jexactyl::nav')
    <div class="row">
        <div class="col-xs-12">
            <form action="{{ route('admin.jexactyl.registration') }}" method="POST">
                <div class="box
                @if($enabled == 'true') box-success @else box-danger @endif">
                    <div class="box-header with-border">
                        <i class="fa fa-at"></i> <h3 class="box-title">Registro via e-mail <small>As configurações para registros e logins por e-mail.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitado</label>
                                <div>
                                    <select name="registration:enabled" class="form-control">
                                        <option @if ($enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    <p class="text-muted"><small>Determina se as pessoas podem registrar contas utilizando e-mails.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box @if($discord_enabled == 'true') box-success @else box-danger @endif">
                    <div class="box-header with-border">
                        <i class="fa fa-comments-o"></i> <h3 class="box-title">Registro via Discord <small>As configurações para registro e logins do Discord.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitado</label>
                                <div>
                                    <select name="discord:enabled" class="form-control">
                                        <option @if ($discord_enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($discord_enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    @if($discord_enabled != 'true')
                                        <p class="text-danger">As pessoas não poderão se registrar ou fazer login com o Discord se isso for desativado!</p>
                                    @else
                                        <p class="text-muted"><small>Determina se as pessoas poderão registrar-se utilizando o Discord.</small></p>
                                    @endif
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Discord Client ID</label>
                                <div>
                                    <input type="text" class="form-control" name="discord:id" value="{{ $discord_id }}" />
                                    <p class="text-muted"><small>O ID do cliente para sua aplicação OAuth. Normalmente com 18-19 números.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Discord Client Secret</label>
                                <div>
                                    <input type="password" class="form-control" name="discord:secret" value="{{ $discord_secret }}" />
                                    <p class="text-muted"><small>O segredo do cliente para sua aplicação OAuth. Trate isto como uma senha.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box box-info">
                    <div class="box-header with-border">
                        <h3 class="box-title"><i class="fa fa-envelope"></i> Verificação de e-mail <small>Ative isto para permitir a verificação por e-mail.</small></h3>
                    </div>
                    <div class="box-body row">
                        <div class="form-group col-md-4">
                            <label for="verification" class="control-label">Status</label>
                            <select name="registration:verification" id="verification" class="form-control">
                                <option value="{{ 1 }}" @if ($verification) selected @endif>Habilitado</option>
                                <option value="{{ 0 }}" @if (!$verification) selected @endif>Desabilitado</option>
                            </select>
                        </div>
                    </div>
                </div>
                <div class="box box-info">
                    <div class="box-header with-border">
                        <i class="fa fa-microchip"></i> <h3 class="box-title">Recursos Padrão <small>Os recursos padrões atribuídos a um usuário no registro.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de CPU</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:cpu" value="{{ $cpu }}" />
                                    <p class="text-muted"><small>A quantidade de CPU que deve ser entregue a um usuário no momento do registro em %..</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de RAM</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:memory" value="{{ $memory }}" />
                                    <p class="text-muted"><small>A quantidade de RAM que deve ser entregue a um usuário no momento do registro em MB.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de armazenamento</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:disk" value="{{ $disk }}" />
                                    <p class="text-muted"><small>A quantidade de armazenamento que deve ser entregue a um usuário no momento do registro em MB.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de slots</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:slot" value="{{ $slot }}" />
                                    <p class="text-muted"><small>A quantidade de slots de servidor que deve ser entregue a um usuário no momento do registro.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de alocações</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:port" value="{{ $port }}" />
                                    <p class="text-muted"><small>A quantidade de portas de servidor que deve ser entregue a um usuário no momento do registro.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de backups</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:backup" value="{{ $backup }}" />
                                    <p class="text-muted"><small>A quantidade de backups de servidor que deve ser entregue a um usuário no momento do registro.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de Banco de Dados</label>
                                <div>
                                    <input type="text" class="form-control" name="registration:database" value="{{ $database }}" />
                                    <p class="text-muted"><small>A quantidade de bancos de dados de servidor que deve ser entregue a um usuário no momento do registro.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                {!! csrf_field() !!}
                <button type="submit" name="_method" value="PATCH" class="btn btn-default pull-right">Salvar mudanças</button>
            </form>
        </div>
    </div>
@endsection
