@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'store'])

@section('title')
    Loja Jexactyl
@endsection

@section('content-header')
    <h1>Loja Jexactyl<small>Configurar a loja do Jexactyl.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
    @yield('jexactyl::nav')
    <div class="row">
        <div class="col-xs-12">
            <form action="{{ route('admin.jexactyl.store') }}" method="POST">
                <div class="box
                    @if($enabled == 'true')
                        box-success
                    @else
                        box-danger
                    @endif
                ">
                    <div class="box-header with-border">
                        <i class="fa fa-shopping-cart"></i> <h3 class="box-title">Loja do Jexactyl <small>Configurar se determinadas opções da loja estarão habilitadas.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitar Loja</label>
                                <div>
                                    <select name="store:enabled" class="form-control">
                                        <option @if ($enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    <p class="text-muted"><small>Determina se os usuários poderão acessar a interface gráfica da loja.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitar PayPal</label>
                                <div>
                                    <select name="store:paypal:enabled" class="form-control">
                                        <option @if ($paypal_enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($paypal_enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    <p class="text-muted"><small>Determina se os usuários poderão comprar créditos com PayPal.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitar Stripe</label>
                                <div>
                                    <select name="store:stripe:enabled" class="form-control">
                                        <option @if ($stripe_enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($stripe_enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    <p class="text-muted"><small>Determina se os usuários poderão comprar créditos com Stripe.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label" for="store:currency">Nome da moeda</label>
                                <select name="store:currency" id="store:currency" class="form-control">
                                    @foreach ($currencies as $currency)
                                        <option @if ($selected_currency === $currency['code']) selected @endif value="{{ $currency['code'] }}">{{ $currency['name'] }}</option>
                                    @endforeach
                                </select>
                                <p class="text-muted"><small>O nome da moeda que será utilizado no Jexactyl.</small></p>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box box-info">
                    <div class="box-header with-border">
                        <i class="fa fa-money"></i> <h3 class="box-title">Ganhar ociosamente (AFK) <small>Definir configurações para a possibilidade de ganhar créditos passivos.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Habilitado</label>
                                <div>
                                    <select name="earn:enabled" class="form-control">
                                        <option @if ($earn_enabled == 'false') selected @endif value="false">Desabilitado</option>
                                        <option @if ($earn_enabled == 'true') selected @endif value="true">Habilitado</option>
                                    </select>
                                    <p class="text-muted"><small>Determina se os usuários poderão ganhar créditos quando inativos na pagina do Painel.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Quantidade de créditos por minuto</label>
                                <div>
                                    <input type="text" class="form-control" name="earn:amount" value="{{ $earn_amount }}" />
                                    <p class="text-muted"><small>A quantidade de créditos que um usuário receberá por minuto AFK.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box box-info">
                    <div class="box-header with-border">
                        <i class="fa fa-dollar"></i> <h3 class="box-title">Preço dos recursos <small>Definir preços específicos para os recursos.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 50% de CPU</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:cpu" value="{{ $cpu }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total para 50% de CPU.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 1GB de RAM</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:memory" value="{{ $memory }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total de 1GB de RAM.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 1GB de Disco</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:disk" value="{{ $disk }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total de 1GB de disco.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 1 Slot de servidor</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:slot" value="{{ $slot }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total de um slot de servidor.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 1 Alocação de Rede</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:port" value="{{ $port }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total de uma porta.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 1 Backup de Servidor</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:backup" value="{{ $backup }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total de um backup.</small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Custo por 1 Database do servidor</label>
                                <div>
                                    <input type="text" class="form-control" name="store:cost:database" value="{{ $database }}" />
                                    <p class="text-muted"><small>Utilizado para calcular o custo total de um Database.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box box-info">
                    <div class="box-header with-border">
                        <i class="fa fa-area-chart"></i> <h3 class="box-title">Limites de recursos <small>Estabelecer limites para a quantidade de cada recurso com que um servidor pode ser implantado.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Limite de CPU</label>
                                <div>
                                    <div class="input-group">
                                        <input type="text" class="form-control" name="store:limit:cpu" value="{{ $limit_cpu }}" />
                                        <span class="input-group-addon">%</span>
                                    </div>
                                    <p class="text-muted"><small>A quantidade máxima de CPU com que um servidor pode ser implantado. </small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Limite de RAM</label>
                                <div>
                                    <div class="input-group">
                                        <input type="text" class="form-control" name="store:limit:memory" value="{{ $limit_memory }}" />
                                        <span class="input-group-addon">MB</span>
                                    </div>
                                    <p class="text-muted"><small>A quantidade máxima de RAM com que um servidor pode ser implantado. </small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Limite de disco</label>
                                <div>
                                    <div class="input-group">
                                        <input type="text" class="form-control" name="store:limit:disk" value="{{ $limit_disk }}" />
                                        <span class="input-group-addon">MB</span>
                                    </div>
                                    <p class="text-muted"><small>A quantidade máxima de disco com que um servidor pode ser implantado. </small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Limite de alocações de rede</label>
                                <div>
                                    <div class="input-group">
                                        <input type="text" class="form-control" name="store:limit:port" value="{{ $limit_port }}" />
                                        <span class="input-group-addon">portas</span>
                                    </div>
                                    <p class="text-muted"><small>A quantidade máxima de portas (alocações) com as quais um servidor pode ser implantado. </small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Limite de backups</label>
                                <div>
                                    <div class="input-group">
                                        <input type="text" class="form-control" name="store:limit:backup" value="{{ $limit_backup }}" />
                                        <span class="input-group-addon">backups</span>
                                    </div>
                                    <p class="text-muted"><small>A quantidade máxima de backups com que um servidor pode ser implantado. </small></p>
                                </div>
                            </div>
                            <div class="form-group col-md-4">
                                <label class="control-label">Limite de Database</label>
                                <div>
                                    <div class="input-group">
                                        <input type="text" class="form-control" name="store:limit:database" value="{{ $limit_database }}" />
                                        <span class="input-group-addon">Database</span>
                                    </div>
                                    <p class="text-muted"><small>A quantidade máxima de bancos de dados com os quais um servidor pode ser implantado. </small></p>
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
