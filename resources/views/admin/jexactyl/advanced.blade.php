@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'advanced'])

@section('title')
    Avançado
@endsection

@section('content-header')
    <h1>Avançado<small>Configurar configurações avançadas para o Painel.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
    @yield('jexactyl::nav')
        <form action="{{ route('admin.jexactyl.advanced') }}" method="POST">
            <div class="row">
                <div class="col-xs-12">
                    <div class="box">
                        <div class="box-header with-border">
                            <h3 class="box-title">Configurações do painel</h3>
                        </div>
                        <div class="box-body">
                            <div class="row">
                                <div class="form-group col-md-4">
                                    <label class="control-label">Exigir autenticação de 2 fatores</label>
                                    <div>
                                        <div class="btn-group" data-toggle="buttons">
                                            @php
                                                $level = old('pterodactyl:auth:2fa_required', config('pterodactyl.auth.2fa_required'));
                                            @endphp
                                            <label class="btn btn-primary @if ($level == 0) active @endif">
                                                <input type="radio" name="pterodactyl:auth:2fa_required" autocomplete="off" value="0" @if ($level == 0) checked @endif> Não é necessário
                                            </label>
                                            <label class="btn btn-primary @if ($level == 1) active @endif">
                                                <input type="radio" name="pterodactyl:auth:2fa_required" autocomplete="off" value="1" @if ($level == 1) checked @endif> Somente Admin
                                            </label>
                                            <label class="btn btn-primary @if ($level == 2) active @endif">
                                                <input type="radio" name="pterodactyl:auth:2fa_required" autocomplete="off" value="2" @if ($level == 2) checked @endif> Todos os usuários
                                            </label>
                                        </div>
                                        <p class="text-muted"><small>Se ativado, qualquer conta que pertencer ao grupo selecionado deverá ter a autenticação de 2 fatores habilitada para usar o Painel.</small></p>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="row">
                <div class="col-xs-12">
                    <div class="box">
                        <div class="box-header with-border">
                            <h3 class="box-title">reCAPTCHA</h3>
                        </div>
                        <div class="box-body">
                            <div class="row">
                                <div class="form-group col-md-4">
                                    <label class="control-label">Status</label>
                                    <div>
                                        <select class="form-control" name="recaptcha:enabled">
                                            <option value="true">Habilitado</option>
                                            <option value="false" @if(old('recaptcha:enabled', config('recaptcha.enabled')) == '0') selected @endif>Desabilitado</option>
                                        </select>
                                        <p class="text-muted small">Se ativado, os formulários de login e de redefinição de senha farão uma verificação silenciosa do captcha e exibirão um captcha visível, se necessário.</p>
                                    </div>
                                </div>
                                <div class="form-group col-md-4">
                                    <label class="control-label">Chave do site</label>
                                    <div>
                                        <input type="text" required class="form-control" name="recaptcha:website_key" value="{{ old('recaptcha:website_key', config('recaptcha.website_key')) }}">
                                    </div>
                                </div>
                                <div class="form-group col-md-4">
                                    <label class="control-label">Chave secreta</label>
                                    <div>
                                        <input type="text" required class="form-control" name="recaptcha:secret_key" value="{{ old('recaptcha:secret_key', config('recaptcha.secret_key')) }}">
                                        <p class="text-muted small">Utilizado para a comunicação entre seu site e o Google. Não deixe de mantê-lo em segredo.</p>
                                    </div>
                                </div>
                            </div>
                            @if($warning)
                                <div class="row">
                                    <div class="col-xs-12">
                                        <div class="alert alert-warning no-margin">
                                        Atualmente você está usando as chaves reCAPTCHA que foram enviadas com este Painel. Para maior segurança, é recomendado <a href="https://www.google.com/recaptcha/admin">gerar novas chaves reCAPTCHA invisíveis</a> que estejam especificamente ligados ao seu website.
                                        </div>
                                    </div>
                                </div>
                            @endif
                        </div>
                    </div>
                    <div class="box">
                        <div class="box-header with-border">
                            <h3 class="box-title">Conexões HTTP</h3>
                        </div>
                        <div class="box-body">
                            <div class="row">
                                <div class="form-group col-md-6">
                                    <label class="control-label">Tempo limite de conexão</label>
                                    <div>
                                        <input type="number" required class="form-control" name="pterodactyl:guzzle:connect_timeout" value="{{ old('pterodactyl:guzzle:connect_timeout', config('pterodactyl.guzzle.connect_timeout')) }}">
                                        <p class="text-muted small">O tempo em segundos para esperar que uma conexão seja aberta antes de emitir um erro.</p>
                                    </div>
                                </div>
                                <div class="form-group col-md-6">
                                    <label class="control-label">Solicitação de tempo limite</label>
                                    <div>
                                        <input type="number" required class="form-control" name="pterodactyl:guzzle:timeout" value="{{ old('pterodactyl:guzzle:timeout', config('pterodactyl.guzzle.timeout')) }}">
                                        <p class="text-muted small">A quantidade de tempo em segundos para esperar que um pedido seja concluído antes de emitir um erro.</p>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    <div class="box">
                        <div class="box-header with-border">
                            <h3 class="box-title">Criação automática de alocações</h3>
                        </div>
                        <div class="box-body">
                            <div class="row">
                                <div class="form-group col-md-4">
                                    <label class="control-label">Status</label>
                                    <div>
                                        <select class="form-control" name="pterodactyl:client_features:allocations:enabled">
                                            <option value="false">Desabilitado</option>
                                            <option value="true" @if(old('pterodactyl:client_features:allocations:enabled', config('pterodactyl.client_features.allocations.enabled'))) selected @endif>Habilitado</option>
                                        </select>
                                        <p class="text-muted small">Se habilitados, os usuários terão a opção de criar automaticamente novas alocações para seu servidor através do front-end.</p>
                                    </div>
                                </div>
                                <div class="form-group col-md-4">
                                    <label class="control-label">Porta Inicial</label>
                                    <div>
                                        <input type="number" class="form-control" name="pterodactyl:client_features:allocations:range_start" value="{{ old('pterodactyl:client_features:allocations:range_start', config('pterodactyl.client_features.allocations.range_start')) }}">
                                        <p class="text-muted small">A porta de partida no intervalo que pode ser alocado automaticamente.</p>
                                    </div>
                                </div>
                                <div class="form-group col-md-4">
                                    <label class="control-label">Porta Final</label>
                                    <div>
                                        <input type="number" class="form-control" name="pterodactyl:client_features:allocations:range_end" value="{{ old('pterodactyl:client_features:allocations:range_end', config('pterodactyl.client_features.allocations.range_end')) }}">
                                        <p class="text-muted small">A porta final no intervalo que pode ser alocada automaticamente.</p>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                    {{ csrf_field() }}
                    <button type="submit" name="_method" value="PATCH" class="btn btn-default pull-right">Salvar configurações</button>
                </div>
            </div>
        </form>
@endsection
